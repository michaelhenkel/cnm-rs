fn main() {
    build_config();
}

fn build_config(){
    tonic_build::configure()
    .build_client(true)
    .out_dir("src/controllers/crpd/junos/proto")
    .include_file("mod.rs")
    .client_mod_attribute("attrs", "#[cfg(feature = \"client\")]")
    //.client_attribute("ConfigController", "#[derive(PartialEq)]")
    .compile(
        &["src/controllers/crpd/junos/proto/jnx_management_service.proto"],
        &["src/controllers/crpd/junos/proto"],
    ).unwrap();
}
