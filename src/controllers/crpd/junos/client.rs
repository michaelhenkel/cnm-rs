use super::proto::jnx::jet::management::OpCommandGetRequest;
use super::proto::jnx::jet::management::management_client;
use super::proto::jnx::jet::management::ConfigSetRequest;
use super::proto::jnx::jet::management::config_set_request;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use super::junos;

pub struct Client {
    client: management_client::ManagementClient<tonic::transport::Channel>,
}

impl Client{
    pub async fn new(address: String, key: String, pem: String) -> anyhow::Result<Client>{
        let client_identity = Identity::from_pem(pem, key);
        let tls = ClientTlsConfig::new()
            .domain_name("localhost")
            .identity(client_identity);
        let endpoint = tonic::transport::Endpoint::from_shared(format!("https://{}:50051",address))?.
            tls_config(tls)?;
        let client = management_client::ManagementClient::connect(endpoint).await?;
        Ok(Client{
            client,
        })
    }
    pub async fn set(&mut self, config: junos::Configuration) -> anyhow::Result<()>{
        let mut request = ConfigSetRequest::default();
        let json_config = serde_json::to_string(&config)?;
        request.config = Some(config_set_request::Config::JsonConfig(json_config));
        self.client.config_set(request).await?;
        Ok(())
    }
    pub async fn get(&mut self) -> anyhow::Result<Option<String>>{
        let mut op_command_request = OpCommandGetRequest::default();
        op_command_request.command = Some(super::proto::jnx::jet::management::op_command_get_request::Command::XmlCommand("<get-configuration></get-configuration>".to_string()));
        op_command_request.set_out_format(super::proto::jnx::jet::management::OpCommandOutputFormat::OpCommandOutputJson);
        let mut res = self.client.op_command_get(op_command_request).await?.into_inner();
        let msg = res.message().await?;
        if let Some(msg) = msg{
            return Ok(Some(msg.data));
        }
        Ok(None)
    }
}