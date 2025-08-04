fn main() {
    sails_rs::build_wasm();

    sails_rs::build_client::<awesome_sails_services::test::TestProgram>();
}
