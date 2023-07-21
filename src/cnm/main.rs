use cnm_rs::resources;
use cnm_rs::controllers;
use kube::Client;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let mut resource_list = Vec::new();

    let crpd_res = resources::crpd::crpd::CrpdResource::new(client.clone());
    let res: Box<dyn resources::resources::Resource> = Box::new(crpd_res);
    resource_list.push(res);

    let bgp_router_res = resources::bgp_router::BgpRouterResource::new(client.clone());
    let res: Box<dyn resources::resources::Resource> = Box::new(bgp_router_res);
    resource_list.push(res);

    let bgp_router_group_res = resources::bgp_router_group::BgpRouterGroupResource::new(client.clone());
    let res: Box<dyn resources::resources::Resource> = Box::new(bgp_router_group_res);
    resource_list.push(res);

    resources::resources::init_resources(resource_list).await?;

    let mut controller_list = Vec::new();

    let crpd_controller = controllers::crpd::crpd::CrpdController::new(client.clone());
    let ctrl: Box<dyn controllers::controllers::Controller> = Box::new(crpd_controller);
    controller_list.push(ctrl);

    let bgp_router_controller = controllers::bgp_router::BgpRouterController::new(client.clone());
    let ctrl: Box<dyn controllers::controllers::Controller> = Box::new(bgp_router_controller);
    controller_list.push(ctrl);

    let crpd_bgp_router_group_controller = controllers::crpd::bgp_router_group::BgpRouterGroupController::new(client.clone());
    let ctrl: Box<dyn controllers::controllers::Controller> = Box::new(crpd_bgp_router_group_controller);
    controller_list.push(ctrl);

    let junos_configuration_controller = controllers::crpd::junos_configuration::JunosConfigurationController::new(client.clone());
    let ctrl: Box<dyn controllers::controllers::Controller> = Box::new(junos_configuration_controller);
    controller_list.push(ctrl);

    tokio::spawn(async move {
        controllers::controllers::init_controllers(controller_list).await
    }).await??;

    Ok(())
}
