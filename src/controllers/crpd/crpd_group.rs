use crate::controllers::controllers::{
    Controller, Context, ReconcileError, self
};
use crate::controllers::crpd::junos::interface;
use crate::resources::interface::{InterfaceSpec, Interface};
use crate::resources::interface_group::InterfaceGroupSpec;
use crate::resources::resources::Parent;
use crate::resources::routing_instance_group::RoutingInstanceGroup;
use crate::resources::vrrp_group::VrrpGroup;
use crate::resources::{
    crpd::crpd::{Crpd, CrpdStatus},
    crpd::crpd_group::{CrpdGroup, CrpdGroupStatus},
    bgp_router_group::BgpRouterGroup,
    interface_group::InterfaceGroup,
    resources
};
use async_trait::async_trait;
use futures::StreamExt;
use kube::{
    Resource,
    api::Api,
    runtime::{
        controller::{Action, Controller as runtime_controller},
        watcher::Config,
        reflector::ObjectRef
    },
};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::*;
use k8s_openapi::{
    api::{
        apps::v1 as apps_v1,
        core::v1 as core_v1,
        rbac::v1 as rbac_v1,
    },
    apimachinery::pkg::apis::meta::v1 as meta_v1,
};
use regex::Regex;


pub struct CrpdGroupController{
    context: Arc<Context>,
    resource: Api<CrpdGroup>,
}

impl CrpdGroupController{
    pub fn new(context: Arc<Context>) -> Self{
        let resource = Api::all(context.client.clone());
        let context = context.clone();
        CrpdGroupController{context, resource}
    }
    async fn reconcile(g: Arc<CrpdGroup>, ctx: Arc<Context>) ->  Result<Action, ReconcileError> {
        let name = g.meta().name.as_ref().unwrap();
        let namespace = g.meta().namespace.as_ref().unwrap();
        match controllers::get::<CrpdGroup>(g.meta().namespace.as_ref().unwrap(),
            g.meta().name.as_ref().unwrap(),
            ctx.client.clone())
            .await{
            Ok(res) => {
                match res{
                    Some((mut crpd_group, _crpd_api)) => {
                        match controllers::list::<BgpRouterGroup>(namespace,ctx.client.clone(),Some(BTreeMap::from([
                            ("cnm.juniper.net/instanceSelector".to_string(), name.to_string())
                            ]))).await{
                                Ok(res) => {
                                    if let Some((bgp_router_group_list,_)) = res {
                                        let ref_list: Vec<core_v1::LocalObjectReference> = bgp_router_group_list.iter().map(|obj|{
                                            core_v1::LocalObjectReference{
                                                name: Some(obj.meta().name.as_ref().unwrap().clone()),
                                            }
                                        }).collect();
                                        if let Some(status) = crpd_group.status.as_mut(){
                                            status.bgp_router_group_references = Some(ref_list);
                                        } else {
                                            crpd_group.status = Some(CrpdGroupStatus{
                                                bgp_router_group_references: Some(ref_list),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                },
                                Err(e) => return Err(e),
                        };
                        match controllers::list::<RoutingInstanceGroup>(namespace,ctx.client.clone(),Some(BTreeMap::from([
                            ("cnm.juniper.net/instanceSelector".to_string(), name.to_string())
                            ]))).await{
                                Ok(res) => {
                                    if let Some((routing_instance_group_list,_)) = res {
                                        let ref_list: Vec<core_v1::LocalObjectReference> = routing_instance_group_list.iter().map(|obj|{
                                            core_v1::LocalObjectReference{
                                                name: Some(obj.meta().name.as_ref().unwrap().clone()),
                                            }
                                        }).collect();
                                        if let Some(status) = crpd_group.status.as_mut(){
                                            status.routing_instance_group_references = Some(ref_list);
                                        } else {
                                            crpd_group.status = Some(CrpdGroupStatus{
                                                routing_instance_group_references: Some(ref_list),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                },
                                Err(e) => return Err(e),
                        };
                        match controllers::list::<VrrpGroup>(namespace,ctx.client.clone(),Some(BTreeMap::from([
                            ("cnm.juniper.net/instanceSelector".to_string(), name.to_string())
                            ]))).await{
                                Ok(res) => {
                                    if let Some((vrrp_group_list,_)) = res {
                                        let ref_list: Vec<core_v1::LocalObjectReference> = vrrp_group_list.iter().map(|obj|{
                                            core_v1::LocalObjectReference{
                                                name: Some(obj.meta().name.as_ref().unwrap().clone()),
                                            }
                                        }).collect();
                                        if let Some(status) = crpd_group.status.as_mut(){
                                            status.vrrp_group_references = Some(ref_list);
                                        } else {
                                            crpd_group.status = Some(CrpdGroupStatus{
                                                vrrp_group_references: Some(ref_list),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                },
                                Err(e) => return Err(e),
                        };
                        match controllers::list::<InterfaceGroup>(namespace,ctx.client.clone(),Some(BTreeMap::from([
                            ("cnm.juniper.net/instanceSelector".to_string(), name.to_string())
                            ]))).await{
                                Ok(res) => {
                                    if let Some((interface_group_list,_)) = res {
                                        let ref_list: Vec<core_v1::LocalObjectReference> = interface_group_list.iter().map(|obj|{
                                            core_v1::LocalObjectReference{
                                                name: Some(obj.meta().name.as_ref().unwrap().clone()),
                                            }
                                        }).collect();
                                        if let Some(status) = crpd_group.status.as_mut(){
                                            status.interface_group_references = Some(ref_list);
                                        } else {
                                            crpd_group.status = Some(CrpdGroupStatus{
                                                interface_group_references: Some(ref_list),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                },
                                Err(e) => return Err(e),
                        };
                        let sts = apps_v1::StatefulSet::from(crpd_group.clone());
                        let sts_string = serde_json::to_string_pretty(&sts).unwrap();
                        println!("{}", sts_string);
                        match controllers::create_or_update(sts.clone(), ctx.client.clone()).await{
                            Ok(_sts) => {
                                info!("sts created");
                            },
                            Err(e) => {
                                return Err(e);
                            },
                        }
                        match controllers::get::<apps_v1::StatefulSet>(sts.meta().namespace.as_ref().unwrap(),
                            sts.meta().name.as_ref().unwrap(),
                            ctx.client.clone())
                            .await{
                            Ok(res) => {
                                match res{
                                    Some((sts, _)) => {
                                        let status = match crpd_group.clone().status{
                                            Some(mut status) => {
                                                status.stateful_set = sts.status.clone();
                                                status
                                            },
                                            None => {
                                                let status = Some(CrpdGroupStatus{
                                                    stateful_set: sts.status.clone(),
                                                    ..Default::default()
                                                });
                                                status.unwrap()
                                            },
                                        };
                                        crpd_group.status = Some(status.clone());                                      
                                    },
                                    None => {},
                                }
                            },
                            Err(e) => {
                                return Err(e);
                            },
                        }

                        match controllers::list::<core_v1::Pod>(namespace, ctx.client.clone(), Some(BTreeMap::from([(
                            "cnm.juniper.net/instanceSelector".to_string(),
                            name.clone(),
                        )]))).await{
                            Ok(pod_list) => {
                                if let Some((pod_list, _)) = pod_list{
                                    let mut crpd_ref_list = Vec::new();
                                    for pod in &pod_list{
                                        let crpd_name = pod.meta().name.as_ref().unwrap();
                                        let crpd_spec = crpd_group.spec.crpd_template.clone();
                                        let mut crpd = Crpd::new(&crpd_name, crpd_spec);
                                        crpd.metadata.namespace = Some(namespace.to_string());
                                        if crpd.metadata.labels.is_none(){
                                            crpd.metadata.labels = Some(BTreeMap::new());
                                        }
                                        crpd.metadata.labels.as_mut().unwrap().insert("cnm.juniper.net/instanceSelector".to_string(), name.to_string());
                                        if let Err(e) = controllers::create_or_update(crpd, ctx.client.clone()).await{
                                            return  Err(e);
                                        }
                                        let crpd_ref = core_v1::LocalObjectReference{
                                            name: Some(crpd_name.clone())
                                        };
                                        crpd_ref_list.push(crpd_ref);
                                    }
                                    if let Some(status) = crpd_group.status.as_mut(){
                                        status.crpd_references = Some(crpd_ref_list);
                                    } else {
                                        crpd_group.status = Some(CrpdGroupStatus{
                                            crpd_references: Some(crpd_ref_list),
                                            ..Default::default()
                                        });
                                    }
                                }
                            },
                            Err(e) => return Err(e),
                        }

                        if let Some(interface_groups) = &crpd_group.spec.interface_groups{
                            match controllers::list::<Crpd>(namespace, ctx.client.clone(), Some(BTreeMap::from([
                                ("cnm.juniper.net/instanceSelector".to_string(), name.to_string())
                            ]))).await{
                                Ok(res) => {
                                    if let Some((crpd_list,_)) = res{
                                        let mut interface_map = HashSet::new();
                                        for crpd in &crpd_list{
                                            if let Some(status) = &crpd.status{
                                                for (interface_name, _) in &status.interfaces{
                                                    interface_map.insert(interface_name.to_string());
                                                }
                                            }
                                        }
                                        let mut matched_interfaces = Vec::new();
                                        for interface_group_regexp in interface_groups{
                                            match Regex::new(format!("{}", interface_group_regexp).as_str()){
                                                Ok(re) => {
                                                    for interface_name in &interface_map{
                                                        if re.is_match(interface_name){
                                                            matched_interfaces.push(interface_name.to_string())
                                                        }
                                                    }
                                                },
                                                Err(e) => return Err(ReconcileError(anyhow::anyhow!("regex error")))
                                            }
                                        }
                                        for matched_interface in &matched_interfaces{
                                            let interface_group_spec = InterfaceGroupSpec{
                                                interface_name: matched_interface.clone(),
                                                interface_template: InterfaceSpec{
                                                    name: matched_interface.clone(),
                                                    managed: true,
                                                    instance_parent: Some(Parent{
                                                        parent_type: resources::InstanceType::Crpd,
                                                        reference: core_v1::LocalObjectReference { name: Some(name.clone()) }
                                                    }),
                                                    mac: None,
                                                    mtu: None,
                                                    families: None,
                                                    vrrp: None
                                                },
                                            };
                                            let mut interface_group = InterfaceGroup::new(format!("{}-{}-ig", name, matched_interface.clone()).as_str(), interface_group_spec);
                                            interface_group.meta_mut().namespace = Some(namespace.clone());
                                            if let Err(e) = controllers::create_or_update(interface_group, ctx.client.clone()).await{
                                                return Err(e)
                                            }
                                        }
                                    }
                                },
                                Err(e) => return Err(e)
                            }
                        }

                        match controllers::update_status(crpd_group, ctx.client.clone()).await{
                            Ok(_crpd) => {
                            },
                            Err(e) => {
                                return Err(e);
                            },
                        };
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
    fn error_policy(_g: Arc<CrpdGroup>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

#[async_trait]
impl Controller for CrpdGroupController{
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

        let reconcile = |g: Arc<CrpdGroup>, ctx: Arc<Context>| {
            async move { CrpdGroupController::reconcile(g, ctx).await }
        };


        let error_policy = |g: Arc<CrpdGroup>, error: &ReconcileError, ctx: Arc<Context>| {
            CrpdGroupController::error_policy(g, error, ctx)
        };
        runtime_controller::new(self.resource.clone(), Config::default())
            .watches(
                Api::<apps_v1::StatefulSet>::all(self.context.client.clone()),
                Config::default(),
                |sts| {
                    match &sts.meta().labels{
                        Some(labels) => {
                            if labels.contains_key("cnm.juniper.net/instanceType") && labels["cnm.juniper.net/instanceType"] == resources::InstanceType::Crpd.to_string(){
                                Some(ObjectRef::<CrpdGroup>::new(
                                    sts.meta().name.as_ref().unwrap())
                                    .within(sts.meta().namespace.as_ref().unwrap()))
                            } else { None }
                        },
                        None => None
                    }
                }
            )
            .watches(
                Api::<RoutingInstanceGroup>::all(self.context.client.clone()),
                Config::default(),
                |group| {
                        if let Some(labels) = &group.meta().labels{
                            if let Some(instance_type) = labels.get("cnm.juniper.net/instanceType"){
                                if instance_type.contains(&resources::InstanceType::Crpd.to_string()){
                                    if let Some(selector) = labels.get("cnm.juniper.net/instanceSelector"){
                                        return Some(ObjectRef::<CrpdGroup>::new(selector)
                                            .within(group.meta().namespace.as_ref().unwrap()));
                                    }
                                }
                            }
                        }
                        None
                    }
                )
            .watches(
                Api::<BgpRouterGroup>::all(self.context.client.clone()),
                Config::default(),
                |group| {
                        if let Some(labels) = &group.meta().labels{
                            if let Some(instance_type) = labels.get("cnm.juniper.net/instanceType"){
                                if instance_type.contains(&resources::InstanceType::Crpd.to_string()){
                                    if let Some(selector) = labels.get("cnm.juniper.net/instanceSelector"){
                                        return Some(ObjectRef::<CrpdGroup>::new(selector)
                                            .within(group.meta().namespace.as_ref().unwrap()));
                                    }
                                }
                            }
                        }
                        None
                    }
                )
            .watches(
                Api::<InterfaceGroup>::all(self.context.client.clone()),
                Config::default(),
                |group| {
                    if let Some(labels) = &group.meta().labels{
                        if let Some(instance_type) = labels.get("cnm.juniper.net/instanceType"){
                            if instance_type.contains(&resources::InstanceType::Crpd.to_string()){
                                if let Some(selector) = labels.get("cnm.juniper.net/instanceSelector"){
                                    return Some(ObjectRef::<CrpdGroup>::new(selector)
                                        .within(group.meta().namespace.as_ref().unwrap()));
                                }
                            }
                        }
                    }
                    None
                }
            )
            .watches(
                Api::<VrrpGroup>::all(self.context.client.clone()),
                Config::default(),
                |group| {
                    if let Some(labels) = &group.meta().labels{
                        if let Some(instance_type) = labels.get("cnm.juniper.net/instanceType"){
                            if instance_type.contains(&resources::InstanceType::Crpd.to_string()){
                                if let Some(selector) = labels.get("cnm.juniper.net/instanceSelector"){
                                    return Some(ObjectRef::<CrpdGroup>::new(selector)
                                        .within(group.meta().namespace.as_ref().unwrap()));
                                }
                            }
                        }
                    }
                    None
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

impl From<CrpdGroup> for apps_v1::StatefulSet{
    fn from(crpd_group: CrpdGroup) -> Self{
        let mut labels = match crpd_group.metadata.clone().labels{
            Some(labels) => {
                labels
            },
            None => {
                BTreeMap::new()
            }
        };
        labels.insert("app".to_string(), "crpd".to_string());
        labels.insert("cnm.juniper.net/instanceSelector".to_string(), crpd_group.metadata.name.as_ref().unwrap().clone());
        labels.insert("cnm.juniper.net/instanceType".to_string(), resources::InstanceType::Crpd.to_string());

        apps_v1::StatefulSet{
            metadata: meta_v1::ObjectMeta{
                name: Some(crpd_group.metadata.name.as_ref().unwrap().clone()),
                namespace: crpd_group.metadata.namespace,
                labels: Some(labels.clone()),
                owner_references: Some(vec![meta_v1::OwnerReference{
                    api_version: "cnm.juniper.net/v1".to_string(),
                    kind: "CrpdGroup".to_string(),
                    name: crpd_group.metadata.name.as_ref().unwrap().clone(),
                    uid: crpd_group.metadata.uid.as_ref().unwrap().clone(),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            spec: Some(apps_v1::StatefulSetSpec{
                replicas: Some(crpd_group.spec.replicas),
                selector: meta_v1::LabelSelector { 
                    match_expressions: None,
                    match_labels: Some(BTreeMap::from([("cnm.juniper.net/instanceSelector".to_string(), crpd_group.metadata.name.as_ref().unwrap().clone())])),
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
                            image: Some(crpd_group.spec.crpd_template.image),
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
                                image: Some(crpd_group.spec.crpd_template.init_image),
                                command: Some(vec![
                                    "crpd-init".to_string(),
                                ]),
                                env: Some(vec![
                                    core_v1::EnvVar{
                                        name: "CRPD_GROUP".to_string(),
                                        value: Some(crpd_group.metadata.name.as_ref().unwrap().clone()),
                                        ..Default::default()
                                    },
                                    core_v1::EnvVar{
                                        name: "POD_UUID".to_string(),
                                        value_from: Some(core_v1::EnvVarSource{
                                            field_ref: Some(core_v1::ObjectFieldSelector{
                                                field_path: "metadata.uid".to_string(),
                                                ..Default::default()
                                            }),
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
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