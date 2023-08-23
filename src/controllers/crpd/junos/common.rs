use serde::{Deserialize, Serialize};
use garde::Validate;
use schemars::JsonSchema;
use super::interface;

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Unicast {
    #[serde(rename = "rib-group")]
    #[garde(skip)]
    rib_group: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct RouteAttributes {
    #[garde(skip)]
    community: Community,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct Community {
    #[garde(skip)]
    #[serde(rename = "import-action")]
    import_action: String,
    #[garde(skip)]
    #[serde(rename = "export-action")]
    export_action: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, JsonSchema, Clone)]
pub struct VrfTarget {
    #[garde(skip)]
    community: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Root{
    pub configuration: Configuration
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct System{
    #[serde(rename = "root-authentication")]
    pub root_authentication: RootAuthentication,
    pub services: Services
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct RootAuthentication{
    #[serde(rename = "encrypted-password")]
    pub encrypted_password: String
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Services{
    pub ssh: Ssh,
    #[serde(rename = "extension-service")]
    pub extension_service: ExtensionService,
    //pub netconf: Netconf,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ExtensionService{
    #[serde(rename = "request-response")]
    pub request_response: RequestResponse,
    pub traceoptions: TraceOptions,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct TraceOptions{
    pub file: File,
    pub flag: Vec<Flag>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct File{
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Flag{
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct RequestResponse{
    pub grpc: Grpc,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Grpc{
    pub ssl: Ssl,
    //#[serde(rename = "skip-authentication")]
    //#[serde(skip_serializing_if = "Option::is_none")]
    //pub skip_authentication: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Ssl{
    pub port: u32,
    #[serde(rename = "local-certificate")]
    pub local_certificate: Vec<String>,
}


#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Ssh{
    #[serde(rename = "root-login")]
    pub root_login: String,
    pub port: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Netconf{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Security{
    pub certificates: Certificates,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Certificates{
    pub local: Vec<Local>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Local{
    pub name: String,
    pub certificate: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Configuration{
    pub version: String,
    pub system: System,
    pub security: Security,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Interface>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Interface{
    pub interface: Option<Vec<interface::Interface>>,
}

/*
{
    "configuration" : {
        "@" : {
            "junos:commit-seconds" : "1692808529",
            "junos:commit-localtime" : "2023-08-23 16:35:29 UTC",
            "junos:commit-user" : "root"
        },
        "version" : "20230616.051920_builder.r1345402",
        "system" : {
            "root-authentication" : {
                "encrypted-password" : "$2b$10$NaWTxLsRy9G58RWD70xGwehT9gwltmoSfxbQwOFB8siCMcfLDdtQm"
            },
            "services" : {
                "ssh" : {
                    "root-login" : "allow",
                    "port" : 24
                },
                "extension-service" : {
                    "request-response" : {
                        "grpc" : {
                            "ssl" : {
                                "port" : 50052,
                                "local-certificate" : ["grpc"]
                            },
                            "skip-authentication" : [null]
                        }
                    },
                    "traceoptions" : {
                        "file" : {
                            "filename" : "jsd"
                        },
                        "flag" : [
                        {
                            "name" : "all"
                        }
                        ]
                    }
                },
                "netconf" : {
                    "ssh" : [null]
                }
            }
        },
        "security" : {
            "certificates" : {
                "local" : [
                {
                    "name" : "grpc",
                    "certificate" : "-----BEGIN PRIVATE KEY-----\\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgJGkjyA1whLHeii8e\\nmtc7iEZCUFM/k6hTkRFNWppVSv6hRANCAATFC0ULiLIXtLBMUgX0XS7fAcNL4xZF\\ndCIf2MnqH3QqBTofXu+0+7q4uo+OVXQpiel/8EhLU7etslx8iuSftS3x\\n-----END PRIVATE KEY-----\\n-----BEGIN CERTIFICATE-----\\nMIIBQTCB6KADAgECAhUA5Nyiq078VbBdw9g6csdmBiMnwNIwCgYIKoZIzj0EAwIw\\nADAgFw03NTAxMDEwMDAwMDBaGA80MDk2MDEwMTAwMDAwMFowITEfMB0GA1UEAwwW\\ncmNnZW4gc2VsZiBzaWduZWQgY2VydDBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IA\\nBMULRQuIshe0sExSBfRdLt8Bw0vjFkV0Ih/YyeofdCoFOh9e77T7uri6j45VdCmJ\\n6X/wSEtTt62yXHyK5J+1LfGjHDAaMBgGA1UdEQQRMA+CB2NycGQxLTCHBMCoaQIw\\nCgYIKoZIzj0EAwIDSAAwRQIhAIll8/1szSmNhNG5E/CsfJiWfTHaCzj2qyomt6h2\\nGh1NAiBTsajfFO4o5bsmzoRHFLUfncuO/ir5Rj1Z00+5mb5BUA==\\n-----END CERTIFICATE-----\\n"
                }
                ]
            }
        },
        "interfaces" : {
            "interface" : [
            {
                "name" : "lima0",
                "unit" : [
                {
                    "name" : 0,
                    "family" : {
                        "inet" : {
                            "address" : [
                            {
                                "name" : "192.168.105.2/24",
                                "vrrp-group" : [
                                {
                                    "name" : 0,
                                    "virtual-address" : [
                                    {
                                        "name" : "10.0.0.1/24",
                                        "device-name" : "lima0"
                                    }
                                    ],
                                    "unicast" : {
                                        "local-address" : [
                                        {
                                            "name" : "192.168.105.2"
                                        }
                                        ],
                                        "peer-address" : [
                                        {
                                            "name" : "192.168.105.3"
                                        }
                                        ]
                                    }
                                }
                                ]
                            }
                            ]
                        },
                        "inet6" : {
                            "address" : [
                            {
                                "name" : "fd84:3b:532e:6228:5055:55ff:feaf:a38e/64"
                            }
                            ]
                        }
                    }
                }
                ]
            }
            ]
        }
    }
}

*/



