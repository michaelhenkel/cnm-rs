use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::resources::interface;

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct Interface {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mtu: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<Vec<Unit>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<InterfaceWeight>,

}#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct NotifyScript {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "script-name")]
    script_name: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct InterfaceWeight {
    cost: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct InterfaceTrack {
    interface: Vec<Interface>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "notify-master")]
    notify_master: Option<NotifyScript>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "notify-backupo")]
    notify_backup: Option<NotifyScript>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct VirtualAddress {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "device-name")]
    device_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct Unicast {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "local-address")]
    local_address: Option<Vec<Address>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "peer-address")]
    peer_address: Option<Vec<Address>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct VrrpGroup {
    name: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "fast-interval")]
    fast_interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    track: Option<InterfaceTrack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "virtual-address")]
    virtual_address: Option<Vec<VirtualAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unicast: Option<Unicast>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct Address {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "vrrp-group")]
    vrrp_group: Option<Vec<VrrpGroup>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct AddressInet6 {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "vrrp-inet6-group")]
    vrrp_inet6_group: Option<Vec<VrrpGroup>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct Inet {
    address: Vec<Address>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct Inet6 {
    address: Vec<AddressInet6>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct Family {
    #[serde(skip_serializing_if = "Option::is_none")]
    inet: Option<Inet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inet6: Option<Inet6>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct Unit {
    name: u32,
    family: Family,
}

impl From<&interface::Interface> for Interface{
    fn from(interface: &interface::Interface) -> Self {
        let mut intf = Interface::default();
        if let Some(name) = &interface.spec.name{
            intf.name = name.clone();
        }
        if let Some(mtu) = interface.spec.mtu{
            intf.mtu = Some(mtu as u32);
        }
        let mut unit = Unit::default();
        unit.name = 0;
        if let Some(families) = &interface.spec.families{
            for family in families{
                match family{
                    interface::InterfaceFamily::Inet(inet) => {
                        
                        let mut address = Address::default();
                        address.name = inet.address.clone();

                        let vrrp_spec = &interface.spec.vrrp;

                        let vrrp_status = if let Some(status) = &interface.status{
                            &status.vrrp
                        } else {
                            &None
                        };

                        if vrrp_spec.is_some() || vrrp_status.is_some() {

                            let mut vrrp_group = VrrpGroup::default();

                            if let Some(vrrp) = vrrp_status{
                                configure_vrrp_v4(vrrp, &mut vrrp_group);
                                address.vrrp_group = Some(vec![vrrp_group.clone()]);
                            }
                            if let Some(vrrp) = vrrp_spec{
                                configure_vrrp_v4(vrrp, &mut vrrp_group);
                                address.vrrp_group = Some(vec![vrrp_group]);
                            }
                        }
                        
                        let mut family_inet = Inet::default();
                        family_inet.address = vec![address];
                        unit.family.inet = Some(family_inet);
                    },
                    interface::InterfaceFamily::Inet6(inet6) => {
                        let mut address = AddressInet6::default();
                        address.name = inet6.address.clone();
                        let mut family_inet6 = Inet6::default();
                        family_inet6.address = vec![address];
                        unit.family.inet6 = Some(family_inet6);
                    }
                }
            }
        }
        intf.unit = Some(vec![unit]);

        intf
    }
}

fn configure_vrrp_v4<'a>(interface_vrrp: &'a interface::Vrrp, vrrp_group: &'a mut VrrpGroup) -> &'a mut VrrpGroup{
    if let Some(fast_interval) = interface_vrrp.fast_interval{
        vrrp_group.fast_interval = Some(fast_interval as u32);
    }
    if let Some(priority) = interface_vrrp.priority{
        vrrp_group.priority = Some(priority as u32);
    }
    if let Some(track) = &interface_vrrp.track{
        let mut vrrp_track = InterfaceTrack::default();
        vrrp_track.notify_master = Some(NotifyScript{
            script_name: Some(vec![track.notify_master.clone()])
        });
        vrrp_track.notify_backup = Some(NotifyScript {
            script_name: Some(vec![track.notify_backup.clone()])
        });
        if let Some(track_interfaces) = &track.interface{
            for track_interface in track_interfaces{
                let vrrp_track_interface = Interface{
                    name: track_interface.interface.clone(),
                    mtu: None,
                    unit: None,
                    weight: Some(InterfaceWeight { cost: track_interface.weight_cost as u32 })
                };
                vrrp_track.interface.push(vrrp_track_interface);
            }
        }
        vrrp_group.track = Some(vrrp_track);
    }
    if let Some(virtual_address) = &interface_vrrp.virtual_address.v4_address{
        
        match virtual_address{
            interface::VirtualAddressAdress::Address(address) => {
                let mut vrrp_address = VirtualAddress::default();
                if let Some(device_name) = &interface_vrrp.virtual_address.device_name{
                    vrrp_address.device_name = Some(device_name.clone());
                }
                vrrp_address.name = address.clone();
                vrrp_group.virtual_address = Some(vec![vrrp_address]);
            },
            _ => {}
        }
    }
    if let Some(unicast) = &interface_vrrp.unicast{
        let mut vrrp_unicast = Unicast::default();
        if let Some(local_address) = &unicast.local_v4_address{
            vrrp_unicast.local_address = Some(vec![Address{
                name: local_address.clone(),
                vrrp_group: None,
            }])
        }
        if let Some(peer_addresses) = &unicast.peer_v4_list{
            let mut vrrp_peer_address = Vec::new();
            for peer_address in peer_addresses{
                vrrp_peer_address.push(Address{
                    name: peer_address.clone(),
                    vrrp_group: None,
                })
            }
            vrrp_unicast.peer_address = Some(vrrp_peer_address);
        }
        vrrp_group.unicast = Some(vrrp_unicast);
    }
    vrrp_group
}
