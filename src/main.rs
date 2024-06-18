// Licensed to Translating Science PBC under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  Translating Science PBC licenses
// this file to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use actix_files as fs;
use actix_web::{App, web::Data, HttpServer};

use std::env;

use rust_smart_fhir::callback::callback;
use rust_smart_fhir::health::check;
use rust_smart_fhir::index::index;
use rust_smart_fhir::launch::launch;
use rust_smart_fhir::state::State;

fn hostname() -> String {
    let default_hostname = String::from("127.0.0.1");
    
    match env::var_os("FHIR_EXAMPLE_HOSTNAME") {
        Some(hostname_ostr) => match hostname_ostr.into_string() {
            Ok(hostname_str) => hostname_str,
            Err(_) => default_hostname,
        },
        None => default_hostname,
    }
}

fn port() -> u16 {
    let port = 8080;
    
    match env::var_os("FHIR_EXAMPLE_PORT") {
        Some(port_ostr) => match port_ostr.into_string() {
            Ok(port_str) => port_str.parse::<u16>().unwrap_or(port),
            Err(_) => port,
        },
        None => port,
    }
}

fn host_and_port() -> String {
    format!("http://{}:{}", hostname(), port())
}

fn domain() -> String {
    match env::var_os("FHIR_EXAMPLE_DOMAIN") {
	Some(domain_ostr) => match domain_ostr.into_string() {
	    Ok(domain_str) => domain_str,
	    Err(_) => host_and_port()
	}
	None => host_and_port()
    }
}

fn client_id() -> String {
    match env::var_os("FHIR_EXAMPLE_CLIENT_ID") {
	Some(client_id_ostr) => match client_id_ostr.into_string() {
	    Ok(client_id_str) => client_id_str,
	    Err(_) => String::from("rust-smart-fhir")
	}
	None => String::from("rust-smart-fhir")
    }
}

fn client_secret() -> String {
    match env::var_os("FHIR_EXAMPLE_CLIENT_SECRET") {
	Some(client_id_ostr) => match client_id_ostr.into_string() {
	    Ok(client_id_str) => client_id_str,
	    Err(_) => String::from("rust-smart-fhir-secret")
	}
	None => String::from("rust-smart-fhir-secret")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let hostname = hostname();
    let port = port();
    println!("Running on http://{}:{}", hostname, port);

    let state = Data::new(State::new(domain(), client_id(), client_secret()));

    HttpServer::new(move || {
    
	
        App::new()
	    .app_data(state.clone())
            .service(check)
	    .service(callback)
            .service(index)
            .service(launch)
            .service(fs::Files::new("/resources", "./resources").show_files_listing())
            .service(fs::Files::new("/lib", "./lib").show_files_listing())
    })
    .bind((hostname, port))?
    .run()
    .await
}
