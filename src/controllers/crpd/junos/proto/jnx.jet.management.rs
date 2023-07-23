/// \[brief\]: Config path from which to retrieve the configuration data
/// \[detail\]: Config path from which to retrieve the configuration data.
/// The 'id' needs to be set for each path request to help associate the
/// responses to the corresponding path.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConfigPathRequest {
    /// \[brief\]: Identifier for the request
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    /// \[brief\]: Data model path to retrieve
    #[prost(string, tag = "2")]
    pub path: ::prost::alloc::string::String,
}
/// \[brief\]: Configuration commit options
/// \[detail\]: Configuration commit options
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConfigCommit {
    /// \[brief\]: Specify the type of commit operation
    /// \[detail\]: This will specify the type of commit operation
    /// commit operation can be commit or commit-synchronize
    #[prost(enumeration = "ConfigCommitType", tag = "1")]
    pub r#type: i32,
    /// \[brief\]: Specify the comment for the commit log
    #[prost(string, tag = "2")]
    pub comment: ::prost::alloc::string::String,
}
/// \[brief\]: Operational command request type to pass to the OpCommandGet RPC
/// \[detail\]: Operational command request type to pass to the OpCommandGet RPC
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OpCommandGetRequest {
    /// \[brief\]: Output format, default is JSON
    #[prost(enumeration = "OpCommandOutputFormat", tag = "3")]
    pub out_format: i32,
    /// \[brief\]: Command to be executed, represented in the required format
    #[prost(oneof = "op_command_get_request::Command", tags = "1, 2")]
    pub command: ::core::option::Option<op_command_get_request::Command>,
}
/// Nested message and enum types in `OpCommandGetRequest`.
pub mod op_command_get_request {
    /// \[brief\]: Command to be executed, represented in the required format
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Command {
        /// \[brief\]: standard cli command input
        #[prost(string, tag = "1")]
        CliCommand(::prost::alloc::string::String),
        /// \[brief\]: xml command input
        #[prost(string, tag = "2")]
        XmlCommand(::prost::alloc::string::String),
    }
}
/// \[brief\]: Request message for executing an operational command
/// \[detail\]: Request message for executing an operational command
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OpCommandGetResponse {
    /// \[brief\]: RPC execution status information
    #[prost(message, optional, tag = "1")]
    pub status: ::core::option::Option<super::common::RpcStatus>,
    /// \[brief\]: Operation command output
    #[prost(string, tag = "3")]
    pub data: ::prost::alloc::string::String,
}
/// \[brief\]: Request for retrieving configuration data from an ephemeral database
/// \[detail\]: Request for retrieving configuration data from an ephemeral database
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EphemeralConfigGetRequest {
    /// \[brief\]: Encoding format for the returned config
    #[prost(enumeration = "ConfigGetOutputFormat", tag = "1")]
    pub encoding: i32,
    /// \[brief\]: List of configuration paths to retrieve config for
    #[prost(message, repeated, tag = "2")]
    pub config_requests: ::prost::alloc::vec::Vec<ConfigPathRequest>,
    /// \[brief\]: Name of ephemeral configuration database instance
    /// \[detail\]: Name of the Ephemeral configuration database instance to run this
    /// request on. This instance should have been configured previously.
    /// If instance_name is an empty string (default behavior), the default
    /// Ephemeral instance will be used.
    #[prost(string, tag = "3")]
    pub instance_name: ::prost::alloc::string::String,
}
/// \[brief\]: Request type to represent the config responses from a EphemeralConfigGet RPC.
/// \[detail\]: Request type to represent the config responses from a EphemeralConfigGet RPC.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EphemeralConfigGetResponse {
    /// \[brief\]: RPC execution status information
    #[prost(message, optional, tag = "1")]
    pub status: ::core::option::Option<super::common::RpcStatus>,
    /// \[brief\]: List of responses for each configuration path request
    #[prost(message, repeated, tag = "3")]
    pub config_responses: ::prost::alloc::vec::Vec<
        ephemeral_config_get_response::ConfigPathResponse,
    >,
}
/// Nested message and enum types in `EphemeralConfigGetResponse`.
pub mod ephemeral_config_get_response {
    /// \[brief\]: Response to ConfigPathRequest
    /// \[detail\]: Response corresponding to a ConfigPathRequest message sent over the
    /// EphemeralConfigGet RPC
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ConfigPathResponse {
        /// \[brief\]: Status code and message for the corresponding ConfigPathRequest
        #[prost(message, optional, tag = "1")]
        pub status: ::core::option::Option<super::super::common::RpcStatus>,
        /// \[brief\]: Response id corresponding to the ConfigPathRequest id
        /// \[detail\]: Response id corresponding to the ConfigPathRequest id. This helps
        /// map the config data retrieved to the requested config path.
        #[prost(string, tag = "3")]
        pub id: ::prost::alloc::string::String,
        /// \[brief\]: Requested config path
        #[prost(string, tag = "4")]
        pub path: ::prost::alloc::string::String,
        /// \[brief\]: Configuration data for the requested config path
        /// \[detail\]: Configuration data for the requested config path. This data maybe
        /// encoded using the encoding specified in set-data-encoding, or
        /// encoding specified in the request.
        #[prost(string, tag = "5")]
        pub value: ::prost::alloc::string::String,
    }
}
/// \[brief\]: Request type for Ephemeral config database
/// \[detail\]: Request type to represent a group of config operations to be applied to the
/// Ephemeral config database.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EphemeralConfigSetRequest {
    /// \[brief\]: List of config operations to apply together as part of this request
    #[prost(message, repeated, tag = "1")]
    pub config_operations: ::prost::alloc::vec::Vec<
        ephemeral_config_set_request::ConfigOperation,
    >,
    /// \[brief\]: Name of the Ephemeral configuration database instance
    /// \[detail\]: Name of the Ephemeral configuration database instance to run this
    /// request on. This instance should have been configured previously.
    /// If instance_name is an empty string (default behavior), the default
    /// Ephemeral instance will be used.
    #[prost(string, tag = "2")]
    pub instance_name: ::prost::alloc::string::String,
    /// \[brief\]: Enable validation of config
    #[prost(bool, tag = "3")]
    pub validate_config: bool,
    /// \[brief\]: Do a load only operation
    #[prost(bool, tag = "4")]
    pub load_only: bool,
}
/// Nested message and enum types in `EphemeralConfigSetRequest`.
pub mod ephemeral_config_set_request {
    /// \[brief\]: A message to represent a single config operation.
    /// \[detail\]: A message to represent a single config operation.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ConfigOperation {
        /// \[brief\]: Identifier for this config operation
        /// \[detail\]: Identifier for this config operation. The corresponding response
        /// will contain this id to allow clients to map requests to correct
        /// operation responses.
        #[prost(string, tag = "1")]
        pub id: ::prost::alloc::string::String,
        /// \[brief\]: Type of configuration modification
        /// \[detail\]: The type of configuration modification requested for the
        /// corresponding path.Note that some commands, such as 'delete'
        /// do not specify any associated data with the path
        #[prost(enumeration = "super::ConfigOperationType", tag = "2")]
        pub operation: i32,
        /// \[brief\]: Configuration path to apply the operation
        /// \[detail\]: The configuration path to apply the operation to. This currently
        /// only supports the root configuration path "/" and any other string
        /// will throw an error. This implies the input config string must be a
        /// fully qualified config tree relative to the root.
        #[prost(string, tag = "3")]
        pub path: ::prost::alloc::string::String,
        /// \[brief\]: Input configuration data in the relevant format.
        #[prost(oneof = "config_operation::Value", tags = "4, 5")]
        pub value: ::core::option::Option<config_operation::Value>,
    }
    /// Nested message and enum types in `ConfigOperation`.
    pub mod config_operation {
        /// \[brief\]: Input configuration data in the relevant format.
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Value {
            /// \[brief\]: config in xml format
            #[prost(string, tag = "4")]
            XmlConfig(::prost::alloc::string::String),
            /// \[brief\]: config in json format
            #[prost(string, tag = "5")]
            JsonConfig(::prost::alloc::string::String),
        }
    }
}
/// \[brief\]: Request type to represent the config operation
/// \[detail\]: Request type to represent the config operation responses from a
/// EphemeralConfigSet RPC.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EphemeralConfigSetResponse {
    /// \[brief\]: RPC execution status information
    #[prost(message, optional, tag = "1")]
    pub status: ::core::option::Option<super::common::RpcStatus>,
    /// \[brief\]: List of responses for each configuration operation
    #[prost(message, repeated, tag = "3")]
    pub operation_responses: ::prost::alloc::vec::Vec<
        ephemeral_config_set_response::ConfigOperationResponse,
    >,
}
/// Nested message and enum types in `EphemeralConfigSetResponse`.
pub mod ephemeral_config_set_response {
    /// \[brief\]: A message representing response to a single config operation request
    /// \[detail\]: A message representing response to a single config operation request
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ConfigOperationResponse {
        /// \[brief\]: Status code and message for config operation request
        #[prost(message, optional, tag = "1")]
        pub status: ::core::option::Option<super::super::common::RpcStatus>,
        /// \[brief\]: Response id corresponding to the ConfigRequest id
        #[prost(string, tag = "3")]
        pub id: ::prost::alloc::string::String,
    }
}
/// \[brief\]: Request type of config operation
/// \[detail\]: Request type to represent the config operation to be performed on the
/// static Junos config database.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConfigSetRequest {
    /// \[brief\]: Type of config load operation
    #[prost(enumeration = "ConfigLoadType", tag = "4")]
    pub load_type: i32,
    /// \[brief\]: Commit info associated with this config operation
    #[prost(message, optional, tag = "5")]
    pub commit: ::core::option::Option<ConfigCommit>,
    /// \[brief\]: Input configuration data in the relevant format
    #[prost(oneof = "config_set_request::Config", tags = "1, 2, 3")]
    pub config: ::core::option::Option<config_set_request::Config>,
}
/// Nested message and enum types in `ConfigSetRequest`.
pub mod config_set_request {
    /// \[brief\]: Input configuration data in the relevant format
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Config {
        /// \[brief\]: config in xml format
        #[prost(string, tag = "1")]
        XmlConfig(::prost::alloc::string::String),
        /// \[brief\]: config in json format
        #[prost(string, tag = "2")]
        JsonConfig(::prost::alloc::string::String),
        /// \[brief\]: config in text format
        #[prost(string, tag = "3")]
        TextConfig(::prost::alloc::string::String),
    }
}
/// \[brief\]: Request type of config operation
/// \[detail\]: Request type to represent the config operation response from a ConfigSet RPC.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConfigSetResponse {
    /// \[brief\]: RPC execution status information
    #[prost(message, optional, tag = "1")]
    pub status: ::core::option::Option<super::common::RpcStatus>,
}
/// \[brief\]: Output format types for an operational command response
/// \[default\]: OP_COMMAND_OUTPUT_JSON
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum OpCommandOutputFormat {
    /// \[brief\]: JSON format
    OpCommandOutputJson = 0,
    /// \[brief\]: XML format
    OpCommandOutputXml = 1,
    /// \[brief\]: CLI Text format
    OpCommandOutputCli = 2,
}
impl OpCommandOutputFormat {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            OpCommandOutputFormat::OpCommandOutputJson => "OP_COMMAND_OUTPUT_JSON",
            OpCommandOutputFormat::OpCommandOutputXml => "OP_COMMAND_OUTPUT_XML",
            OpCommandOutputFormat::OpCommandOutputCli => "OP_COMMAND_OUTPUT_CLI",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "OP_COMMAND_OUTPUT_JSON" => Some(Self::OpCommandOutputJson),
            "OP_COMMAND_OUTPUT_XML" => Some(Self::OpCommandOutputXml),
            "OP_COMMAND_OUTPUT_CLI" => Some(Self::OpCommandOutputCli),
            _ => None,
        }
    }
}
/// \[brief\]: Encoding format types for the returned configuration data
/// \[default\]: CONFIG_GET_OUTPUT_JSON
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ConfigGetOutputFormat {
    /// \[brief\]: JSON format
    ConfigGetOutputJson = 0,
    /// \[brief\]: XML format
    ConfigGetOutputXml = 1,
}
impl ConfigGetOutputFormat {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ConfigGetOutputFormat::ConfigGetOutputJson => "CONFIG_GET_OUTPUT_JSON",
            ConfigGetOutputFormat::ConfigGetOutputXml => "CONFIG_GET_OUTPUT_XML",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "CONFIG_GET_OUTPUT_JSON" => Some(Self::ConfigGetOutputJson),
            "CONFIG_GET_OUTPUT_XML" => Some(Self::ConfigGetOutputXml),
            _ => None,
        }
    }
}
/// \[brief\]: Type of operation associcated with a configuration set request
/// \[default\]: CONFIG_OPERATION_UPDATE
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ConfigOperationType {
    /// \[brief\]: Update the config
    ConfigOperationUpdate = 0,
}
impl ConfigOperationType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ConfigOperationType::ConfigOperationUpdate => "CONFIG_OPERATION_UPDATE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "CONFIG_OPERATION_UPDATE" => Some(Self::ConfigOperationUpdate),
            _ => None,
        }
    }
}
/// \[brief\]: The load operation type to apply for the configuration set request.
/// \[default\]: CONFIG_LOAD_MERGE
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ConfigLoadType {
    /// \[brief\]: Combines existing configuration with payload
    /// \[detail\]: Combine the configuration that is currently shown in the CLI with the
    /// input configuration (default)
    ConfigLoadMerge = 0,
    /// \[brief\]: Replace parts of existing configuration
    /// \[detail\]: Replace configuration data from the existing configuration with the ones
    /// referred to by the 'replace' tags in the input configuration. These
    /// replace tags come in various flavors depending on the config type:
    /// text -> to be replaced config text is preceded by 'replace: '
    /// xml  -> to be replaced element has attribute operation="replace"
    /// json  -> to be replaced object has metadata tag encoded with '@':
    ///              "object-to-be-replaced": {
    ///                "@": {
    ///                  "operation": "replace"
    ///                }
    ///              }
    ///
    ConfigLoadReplace = 1,
    /// \[brief\]: Replace existing configuration with payload configuration
    /// \[detail\]: Discard the entire existing configuration and load the entire input
    /// configuration. Marks every object as changed.
    ConfigLoadOverride = 2,
    /// \[brief\]: Update existing configuration hierarchies with payload configuration
    /// \[detail\]: Update existing configuration hierarchies with corresponding data from
    /// the input configuration. Marks only affected objects as changed
    ConfigLoadUpdate = 3,
    /// \[brief\]: Load input configuration consisting of set commands
    /// \[detail\]: Load input configuration consisting of set configuration mode commands.
    /// Input config can contain any configuration mode command, such as set,
    /// delete, edit, exit, and top.
    ConfigLoadSet = 4,
}
impl ConfigLoadType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ConfigLoadType::ConfigLoadMerge => "CONFIG_LOAD_MERGE",
            ConfigLoadType::ConfigLoadReplace => "CONFIG_LOAD_REPLACE",
            ConfigLoadType::ConfigLoadOverride => "CONFIG_LOAD_OVERRIDE",
            ConfigLoadType::ConfigLoadUpdate => "CONFIG_LOAD_UPDATE",
            ConfigLoadType::ConfigLoadSet => "CONFIG_LOAD_SET",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "CONFIG_LOAD_MERGE" => Some(Self::ConfigLoadMerge),
            "CONFIG_LOAD_REPLACE" => Some(Self::ConfigLoadReplace),
            "CONFIG_LOAD_OVERRIDE" => Some(Self::ConfigLoadOverride),
            "CONFIG_LOAD_UPDATE" => Some(Self::ConfigLoadUpdate),
            "CONFIG_LOAD_SET" => Some(Self::ConfigLoadSet),
            _ => None,
        }
    }
}
/// \[brief\]: Type of commit to run after loading the configuration
/// \[default\]: CONFIG_COMMIT
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ConfigCommitType {
    /// \[brief\]: Regular commit on current routing engine
    ConfigCommit = 0,
    /// \[brief\]: Sync and commit config to both routing engines
    ConfigCommitSynchronize = 1,
}
impl ConfigCommitType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ConfigCommitType::ConfigCommit => "CONFIG_COMMIT",
            ConfigCommitType::ConfigCommitSynchronize => "CONFIG_COMMIT_SYNCHRONIZE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "CONFIG_COMMIT" => Some(Self::ConfigCommit),
            "CONFIG_COMMIT_SYNCHRONIZE" => Some(Self::ConfigCommitSynchronize),
            _ => None,
        }
    }
}
/// Generated client implementations.
pub mod management_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// [brief]: Junos configuration and operational management service.
    /// [detail]: Junos configuration and operational management service.
    #[derive(Debug, Clone)]
    pub struct ManagementClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ManagementClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ManagementClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ManagementClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ManagementClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// [brief]: Run an operational command
        /// [detail]: This RPC returns the operational command output as a streamed response
        pub async fn op_command_get(
            &mut self,
            request: impl tonic::IntoRequest<super::OpCommandGetRequest>,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::OpCommandGetResponse>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/jnx.jet.management.Management/OpCommandGet",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("jnx.jet.management.Management", "OpCommandGet"),
                );
            self.inner.server_streaming(req, path, codec).await
        }
        /// [brief]: Perform configuration operation on static database
        /// [detail]: Load and commit configuration onto a Junos device
        pub async fn config_set(
            &mut self,
            request: impl tonic::IntoRequest<super::ConfigSetRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ConfigSetResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/jnx.jet.management.Management/ConfigSet",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("jnx.jet.management.Management", "ConfigSet"));
            self.inner.unary(req, path, codec).await
        }
        /// [brief]: Retrieve epehemral configuration from the device
        /// [detail]: Retrieve epehemral configuration from the device
        pub async fn ephemeral_config_get(
            &mut self,
            request: impl tonic::IntoRequest<super::EphemeralConfigGetRequest>,
        ) -> std::result::Result<
            tonic::Response<super::EphemeralConfigGetResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/jnx.jet.management.Management/EphemeralConfigGet",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "jnx.jet.management.Management",
                        "EphemeralConfigGet",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// [brief]: Perform configuration operation on the ephemeral database
        /// [detail]: Load and commit configuration onto Junos device's epehemral database
        pub async fn ephemeral_config_set(
            &mut self,
            request: impl tonic::IntoRequest<super::EphemeralConfigSetRequest>,
        ) -> std::result::Result<
            tonic::Response<super::EphemeralConfigSetResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/jnx.jet.management.Management/EphemeralConfigSet",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "jnx.jet.management.Management",
                        "EphemeralConfigSet",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod management_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ManagementServer.
    #[async_trait]
    pub trait Management: Send + Sync + 'static {
        /// Server streaming response type for the OpCommandGet method.
        type OpCommandGetStream: futures_core::Stream<
                Item = std::result::Result<super::OpCommandGetResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// [brief]: Run an operational command
        /// [detail]: This RPC returns the operational command output as a streamed response
        async fn op_command_get(
            &self,
            request: tonic::Request<super::OpCommandGetRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::OpCommandGetStream>,
            tonic::Status,
        >;
        /// [brief]: Perform configuration operation on static database
        /// [detail]: Load and commit configuration onto a Junos device
        async fn config_set(
            &self,
            request: tonic::Request<super::ConfigSetRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ConfigSetResponse>,
            tonic::Status,
        >;
        /// [brief]: Retrieve epehemral configuration from the device
        /// [detail]: Retrieve epehemral configuration from the device
        async fn ephemeral_config_get(
            &self,
            request: tonic::Request<super::EphemeralConfigGetRequest>,
        ) -> std::result::Result<
            tonic::Response<super::EphemeralConfigGetResponse>,
            tonic::Status,
        >;
        /// [brief]: Perform configuration operation on the ephemeral database
        /// [detail]: Load and commit configuration onto Junos device's epehemral database
        async fn ephemeral_config_set(
            &self,
            request: tonic::Request<super::EphemeralConfigSetRequest>,
        ) -> std::result::Result<
            tonic::Response<super::EphemeralConfigSetResponse>,
            tonic::Status,
        >;
    }
    /// [brief]: Junos configuration and operational management service.
    /// [detail]: Junos configuration and operational management service.
    #[derive(Debug)]
    pub struct ManagementServer<T: Management> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Management> ManagementServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ManagementServer<T>
    where
        T: Management,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/jnx.jet.management.Management/OpCommandGet" => {
                    #[allow(non_camel_case_types)]
                    struct OpCommandGetSvc<T: Management>(pub Arc<T>);
                    impl<
                        T: Management,
                    > tonic::server::ServerStreamingService<super::OpCommandGetRequest>
                    for OpCommandGetSvc<T> {
                        type Response = super::OpCommandGetResponse;
                        type ResponseStream = T::OpCommandGetStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::OpCommandGetRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).op_command_get(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = OpCommandGetSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/jnx.jet.management.Management/ConfigSet" => {
                    #[allow(non_camel_case_types)]
                    struct ConfigSetSvc<T: Management>(pub Arc<T>);
                    impl<
                        T: Management,
                    > tonic::server::UnaryService<super::ConfigSetRequest>
                    for ConfigSetSvc<T> {
                        type Response = super::ConfigSetResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ConfigSetRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move { (*inner).config_set(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ConfigSetSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/jnx.jet.management.Management/EphemeralConfigGet" => {
                    #[allow(non_camel_case_types)]
                    struct EphemeralConfigGetSvc<T: Management>(pub Arc<T>);
                    impl<
                        T: Management,
                    > tonic::server::UnaryService<super::EphemeralConfigGetRequest>
                    for EphemeralConfigGetSvc<T> {
                        type Response = super::EphemeralConfigGetResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EphemeralConfigGetRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).ephemeral_config_get(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EphemeralConfigGetSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/jnx.jet.management.Management/EphemeralConfigSet" => {
                    #[allow(non_camel_case_types)]
                    struct EphemeralConfigSetSvc<T: Management>(pub Arc<T>);
                    impl<
                        T: Management,
                    > tonic::server::UnaryService<super::EphemeralConfigSetRequest>
                    for EphemeralConfigSetSvc<T> {
                        type Response = super::EphemeralConfigSetResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EphemeralConfigSetRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).ephemeral_config_set(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EphemeralConfigSetSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Management> Clone for ManagementServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: Management> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Management> tonic::server::NamedService for ManagementServer<T> {
        const NAME: &'static str = "jnx.jet.management.Management";
    }
}
