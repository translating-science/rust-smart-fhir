# Example Rust-based SMART-on-FHIR app

This is a Rust-based SMART-on-FHIR application that recreates [Cerner's SMART-on-FHIR tutorial](https://engineering.cerner.com/smart-on-fhir-tutorial/#standalone-app-launch-for-patient-access-workflow)
using [Actix](https://actix.rs) and [Maud](https://maud.lambda.xyz).

## License

This repository is licensed Apache 2.0, with several exceptions:

* This repository packages in several Javascript libraries, under `lib`
* Two CSS and JS resource files are copied in verbatim from [Cerner's tutorial repo](https://github.com/cerner/smart-on-fhir-tutorial/tree/gh-pages),
  these files are under `resources`
