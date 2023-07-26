use std::f32::consts::E;

use super::proto::jnx::jet::management as junos_mgmt;
use super::proto::jnx::jet::authentication as junos_auth;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use tonic::Request;
use tonic::metadata::MetadataMap;
use tracing::info;
use super::junos;
use tracing::warn;

pub struct Client {
    client: junos_mgmt::management_client::ManagementClient<tonic::transport::Channel>,
}

impl Client{
    pub async fn new(address: String, domain_name: String, key: String, ca: String, cert: String) -> anyhow::Result<Client>{
        //let client_identity = Identity::from_pem(cert, key);
        let mut map = MetadataMap::new();
        map.insert("client-id", "cnm".parse().unwrap());
        let tls = ClientTlsConfig::new()
            .domain_name(domain_name)
            .ca_certificate(Certificate::from_pem(ca));

        let ep_address = format!("http://{}:50052",address);
        info!("Connecting to {}", ep_address);
        let channel = Channel::from_shared(ep_address)?
            .tls_config(tls)?
            .connect()
            .await?;

        //c := auth.NewLoginClient(conn)
        let login_request = junos_auth::LoginRequest{
            username: "root".to_string(),
            password: "Juniper123".to_string(),
            group_id: "cnm".to_string(),
            client_id: "cnm".to_string(),
        };

        let login_response = match junos_auth::authentication_client::AuthenticationClient::new(channel.clone()).login(login_request).await{
            Ok(res) => {
                res
            },
            Err(e) => {
                return Err(e.into())
            }
        };

        info!("login response: {:#?}", login_response.into_inner());

        let client = junos_mgmt::management_client::ManagementClient::new(channel);

        Ok(Client{
            client,
        })
    }
    pub async fn set(&mut self, config: junos::Configuration) -> anyhow::Result<()>{
        let mut request = junos_mgmt::ConfigSetRequest::default();
        let json_config = serde_json::to_string(&config)?;
        request.config = Some(junos_mgmt::config_set_request::Config::JsonConfig(json_config));
        self.client.config_set(request).await?;
        Ok(())
    }
    pub async fn get(&mut self) -> anyhow::Result<Option<String>>{
        let mut op_command_request = junos_mgmt::OpCommandGetRequest::default();
        op_command_request.command = Some(junos_mgmt::op_command_get_request::Command::XmlCommand("<get-configuration></get-configuration>".to_string()));
        op_command_request.set_out_format(junos_mgmt::OpCommandOutputFormat::OpCommandOutputJson);
        //op_command_request.command = Some(junos_mgmt::op_command_get_request::Command::CliCommand("show configuration".to_string()));
        //op_command_request.set_out_format(junos_mgmt::OpCommandOutputFormat::OpCommandOutputCli);
        let mut request = Request::new(op_command_request);
        request.metadata_mut().insert("client-id", "cnm".parse().unwrap());

        let mut response = match self.client
        .op_command_get(request).await{
            Ok(stream) => {
                stream.into_inner()
            },
            Err(e) => {
                warn!("streaming error {}",e);
                return Err(e.into());
            }
        };

        let msg = match response.message().await{
            Ok(msg) => { msg },
            Err(e) => {
                warn!("message error {}",e);
                return Err(e.into());
            }
        };
        if let Some(msg) = msg{
            info!("got config {:#?}", msg);
            return Ok(Some(msg.data));
        }
        info!("got empty config {:#?}", msg);
        Ok(None)
    }
}