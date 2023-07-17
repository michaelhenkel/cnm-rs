use cnm_rs::resources;
use cnm_rs::controllers;
use kube::Client;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let mut resource_list = Vec::new();

    let ri_res = resources::routing_instance::RoutingInstanceResource::new(client.clone());
    let res: Box<dyn resources::resources::Resource> = Box::new(ri_res);
    resource_list.push(res);

    resources::resources::init_resources(resource_list).await?;

    let mut controller_list = Vec::new();

    let ri_controller = controllers::routing_instance::RoutingInstanceController::new(client.clone());
    let ctrl: Box<dyn controllers::controllers::Controller> = Box::new(ri_controller);
    controller_list.push(ctrl);

    tokio::spawn(async move {
        controllers::controllers::init_controllers(controller_list).await
    }).await??;

    Ok(())
}
