use cnm_rs::resources;
use cnm_rs::controllers;
use cnm_rs::admission;
use kube::Client;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
    .event_format(
        tracing_subscriber::fmt::format()
            .with_file(true)
            .with_line_number(true)
    )
    .init();
    let client = Client::try_default().await?;

    let mut join_handlers = Vec::new();

    let adm = admission::admission::AdmissionController::new("192.168.105.4".to_string(),"cnm-admission-controller".to_string(), client.clone());
    
    join_handlers.push(tokio::spawn(async move {
        adm.admission().await
    }));

    let resource_list: Vec<Box<dyn resources::resources::Resource>> = vec![

        Box::new(resources::crpd::crpd::CrpdResource::new(client.clone())),
        Box::new(resources::bgp_router::BgpRouterResource::new(client.clone())),
        Box::new(resources::bgp_router_group::BgpRouterGroupResource::new(client.clone())),

    ];
    resources::resources::init_resources(resource_list).await?;

    let controller_list: Vec<Box<dyn controllers::controllers::Controller>> = vec![

        Box::new(controllers::crpd::crpd::CrpdController::new(client.clone())),
        Box::new(controllers::bgp_router::BgpRouterController::new(client.clone())),
        Box::new(controllers::crpd::bgp_router_group::BgpRouterGroupController::new(client.clone())),
        Box::new(controllers::crpd::junos_configuration::JunosConfigurationController::new(client.clone())),

    ];
    

    join_handlers.push(
        tokio::spawn(async move {
            controllers::controllers::init_controllers(controller_list).await
        })
    );

    futures::future::join_all(join_handlers).await;

    Ok(())
}
