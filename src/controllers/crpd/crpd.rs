use crate::controllers::controllers::{Controller, Context, ReconcileError, self};
use crate::resources::crpd::crpd::{Crpd, CrpdStatus, Instance};
use async_trait::async_trait;
use futures::StreamExt;
use kube::Resource;
use kube::runtime::reflector::ObjectRef;
use kube::{
    api::Api,
    client::Client,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
    },
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::api::apps::v1 as apps_v1;
use k8s_openapi::api::core::v1 as core_v1;
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as meta_v1;
use k8s_openapi::api::rbac::v1 as rbac_v1;

//rbac.authorization.k8s.io/v1

pub struct CrpdController{
    context: Arc<Context>,
    resource: Api<Crpd>,
}

impl CrpdController{
    pub fn new(client: Client) -> Self{
        let resource = Api::all(client.clone());
        let context = Arc::new(Context{
            client: client.clone(),
        });
        CrpdController{context, resource}
    }
    async fn reconcile(g: Arc<Crpd>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        match controllers::get::<Crpd>(g.meta().namespace.as_ref().unwrap().clone(),
            g.meta().name.as_ref().unwrap().clone(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut crpd, _crpd_api)) => {
                        let sts = apps_v1::StatefulSet::from(crpd.clone());
                        match controllers::create_or_update(sts.clone(), ctx.client.clone()).await{
                            Ok(sts) => {
                                info!("sts created");
                            },
                            Err(e) => {
                                return Err(e);
                            },
                        }
                        match controllers::get::<apps_v1::StatefulSet>(sts.meta().namespace.as_ref().unwrap().clone(),
                            sts.meta().name.as_ref().unwrap().clone(),
                            ctx.client.clone())
                            .await{
                            Ok(res) => {
                                match res{
                                    Some((sts, _)) => {
                                        let status = match crpd.clone().status{
                                            Some(mut status) => {
                                                status.stateful_set = sts.status.clone();
                                                status
                                            },
                                            None => {
                                                let status = Some(CrpdStatus{
                                                    stateful_set: sts.status.clone(),
                                                    ..Default::default()
                                                });
                                                status.unwrap()
                                            },
                                        };
                                        crpd.status = Some(status.clone());
                                        let labels = BTreeMap::from_iter(vec![
                                            ("app".to_string(), "crpd".to_string()),
                                            ("crpd".to_string(), crpd.metadata.name.as_ref().unwrap().clone()),
                                        ]);
                                        let mut instances = Vec::new();
                                        match controllers::list::<core_v1::Pod>(
                                            sts.meta().namespace.as_ref().unwrap().as_str(),
                                            ctx.client.clone(),
                                            Some(labels))
                                            .await{
                                                Ok(pod_list) => {
                                                    match pod_list {
                                                        Some((pod_list, _)) => {
                                                            for pod in pod_list.items{
                                                                if pod.status.as_ref().unwrap().pod_ip.is_some(){
                                                                    let instance = Instance{
                                                                        name: pod.meta().name.as_ref().unwrap().clone(),
                                                                        address: pod.status.as_ref().unwrap().pod_ip.as_ref().unwrap().clone(),
                                                                        uuid: pod.meta().uid.as_ref().unwrap().clone(),
                                                                    };
                                                                    instances.push(instance);
                                                                }
                                                            }
                                                        },
                                                        None => {},
                                                    }
                                                },
                                                Err(e) => {
                                                    return Err(e);
                                                },
                                        }
                                        crpd.status.as_mut().unwrap().instances = Some(instances);                                        
                                        match controllers::update_status(crpd, ctx.client.clone()).await{
                                            Ok(crpd) => {
                                            },
                                            Err(e) => {
                                                return Err(e);
                                            },
                                        }
                                    },
                                    None => {},
                                }
                            },
                            Err(e) => {
                                return Err(e);
                            },
                        }
                        Ok(Action::await_change())
                    },
                    None => {
                        info!("crpd does not exist");
                        Ok(Action::await_change())
                    },
                }
            },
            Err(e) => {
                return Err(e);
            },
        }
    }
    fn error_policy(g: Arc<Crpd>, error: &ReconcileError, ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for CrpdController{
    async fn run(&self) -> anyhow::Result<()>{
        let role = rbac_v1::Role{
            metadata: meta_v1::ObjectMeta{
                name: Some("crpd".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            rules: Some(vec![rbac_v1::PolicyRule{
                api_groups: Some(vec!["*".to_string()]),
                resources: Some(vec!["*".to_string()]),
                verbs: vec!["*".to_string()],
                ..Default::default()
            }]),
            ..Default::default()   
        };
        controllers::create_or_update(role, self.context.client.clone()).await?;

        let service_account = core_v1::ServiceAccount{
            metadata: meta_v1::ObjectMeta {
                name: Some("crpd".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            automount_service_account_token: Some(true),
            ..Default::default()
        };
        controllers::create_or_update(service_account, self.context.client.clone()).await?;

        let role_binding = rbac_v1::RoleBinding{
            metadata: meta_v1::ObjectMeta{
                name: Some("crpd".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            role_ref: rbac_v1::RoleRef{
                api_group: "rbac.authorization.k8s.io".to_string(),
                kind: "Role".to_string(),
                name: "crpd".to_string(),
            },
            subjects: Some(vec![rbac_v1::Subject{
                kind: "ServiceAccount".to_string(),
                name: "crpd".to_string(),
                namespace: Some("default".to_string()),
                ..Default::default()
            }]),
        };
        controllers::create_or_update(role_binding, self.context.client.clone()).await?;

        let reconcile = |g: Arc<Crpd>, ctx: Arc<Context>| {
            async move { CrpdController::reconcile(g, ctx).await }
        };


        let error_policy = |g: Arc<Crpd>, error: &ReconcileError, ctx: Arc<Context>| {
            CrpdController::error_policy(g, error, ctx)
        };
        runtime_controller::new(self.resource.clone(), Config::default())
            .watches(
                Api::<apps_v1::StatefulSet>::all(self.context.client.clone()),
                Config::default(),
                |sts| {
                    match &sts.meta().labels{
                        Some(labels) => {
                            if labels.contains_key("app") && labels["app"] == "crpd"{
                                Some(ObjectRef::<Crpd>::new(
                                    sts.meta().name.as_ref().unwrap())
                                    .within(sts.meta().namespace.as_ref().unwrap()))
                            } else {
                                None
                            }
                        },
                        None => {
                            None
                        },
                    }
                }
            )
            .run(reconcile, error_policy, self.context.clone())
            .for_each(|res| async move {
                match res {
                    Ok(o) => info!("reconciled {:?}", o),
                    Err(e) => warn!("reconcile failed: {:?}", e),
                }
            })
            .await;
        Ok(())
    }
}

impl From<Crpd> for apps_v1::StatefulSet{
    fn from(crpd: Crpd) -> Self{
        let mut labels = match crpd.metadata.clone().labels{
            Some(labels) => {
                labels
            },
            None => {
                BTreeMap::new()
            }
        };
        labels.insert("app".to_string(), "crpd".to_string());
        labels.insert("crpd".to_string(), crpd.metadata.name.as_ref().unwrap().clone());

        apps_v1::StatefulSet{
            metadata: meta_v1::ObjectMeta{
                name: Some(crpd.metadata.name.as_ref().unwrap().clone()),
                namespace: crpd.metadata.namespace,
                labels: Some(labels.clone()),
                owner_references: Some(vec![meta_v1::OwnerReference{
                    api_version: "cnm.juniper.net/v1".to_string(),
                    kind: "Crpd".to_string(),
                    name: crpd.metadata.name.as_ref().unwrap().clone(),
                    uid: crpd.metadata.uid.as_ref().unwrap().clone(),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            spec: Some(apps_v1::StatefulSetSpec{
                replicas: Some(crpd.spec.replicas),
                selector: meta_v1::LabelSelector { 
                    match_expressions: None,
                    match_labels: Some(BTreeMap::from([("crpd".to_string(), crpd.metadata.name.as_ref().unwrap().clone())])),
                },
                template: core_v1::PodTemplateSpec { 
                    metadata: Some(meta_v1::ObjectMeta {
                        labels: Some(labels),
                        ..Default::default()
                     }),
                    spec: Some(core_v1::PodSpec{
                        volumes: Some(vec![
                            core_v1::Volume{
                                name: "certs".to_string(),
                                empty_dir: Some(core_v1::EmptyDirVolumeSource{
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            core_v1::Volume{
                                name: "config".to_string(),
                                empty_dir: Some(core_v1::EmptyDirVolumeSource{
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        ]),
                        service_account_name: Some("crpd".to_string()),
                        host_network: Some(true),
                        tolerations: Some(vec![core_v1::Toleration{
                            effect: Some("NoSchedule".to_string()),
                            key: Some("node-role.kubernetes.io/master".to_string()),
                            operator: Some("Exists".to_string()),
                            ..Default::default()
                        }]),
                        containers: vec![
                            core_v1::Container{
                            ports: Some(vec![core_v1::ContainerPort{
                                container_port: 179,
                                ..Default::default()
                            }]),
                            name: "crpd".to_string(),
                            image: Some(crpd.spec.image),
                            security_context: Some(core_v1::SecurityContext{
                                privileged: Some(true),
                                ..Default::default()
                            }),
                            volume_mounts: Some(vec![
                                core_v1::VolumeMount{
                                    name: "certs".to_string(),
                                    mount_path: "/etc/certs".to_string(),
                                    ..Default::default()
                                },
                                core_v1::VolumeMount{
                                    name: "config".to_string(),
                                    mount_path: "/config".to_string(),
                                    ..Default::default()
                                },
                            ]),
                            ..Default::default()
                        },
                        ],
                        init_containers: Some(vec![
                            core_v1::Container{
                                name: "init".to_string(),
                                image: Some(crpd.spec.init_image),
                                command: Some(vec![
                                    "crpd-init".to_string(),
                                ]),
                                env: Some(vec![
                                    core_v1::EnvVar{
                                        name: "POD_IP".to_string(),
                                        value_from: Some(core_v1::EnvVarSource{
                                            field_ref: Some(core_v1::ObjectFieldSelector{
                                                field_path: "status.podIP".to_string(),
                                                ..Default::default()
                                            }),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    core_v1::EnvVar{
                                        name: "POD_NAME".to_string(),
                                        value_from: Some(core_v1::EnvVarSource{
                                            field_ref: Some(core_v1::ObjectFieldSelector{
                                                field_path: "metadata.name".to_string(),
                                                ..Default::default()
                                            }),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    core_v1::EnvVar{
                                        name: "POD_NAMESPACE".to_string(),
                                        value_from: Some(core_v1::EnvVarSource{
                                            field_ref: Some(core_v1::ObjectFieldSelector{
                                                field_path: "metadata.namespace".to_string(),
                                                ..Default::default()
                                            }),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    core_v1::EnvVar{
                                        name: "POD_UID".to_string(),
                                        value_from: Some(core_v1::EnvVarSource{
                                            field_ref: Some(core_v1::ObjectFieldSelector{
                                                field_path: "metadata.uid".to_string(),
                                                ..Default::default()
                                            }),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                ]),
                                security_context: Some(core_v1::SecurityContext{
                                    privileged: Some(true),
                                    ..Default::default()
                                }),
                                volume_mounts: Some(vec![
                                    core_v1::VolumeMount{
                                        name: "certs".to_string(),
                                        mount_path: "/etc/certs".to_string(),
                                        ..Default::default()
                                    },
                                    core_v1::VolumeMount{
                                        name: "config".to_string(),
                                        mount_path: "/config".to_string(),
                                        ..Default::default()
                                    },
                                ]),
                                ..Default::default()
                            }
                        ]),
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}