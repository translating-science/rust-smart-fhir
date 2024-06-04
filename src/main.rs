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
use actix_web::{App, HttpServer};

use rust_smart_fhir::launch::launch;
use rust_smart_fhir::index::index;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let hostname = String::from("localhost");
    let port: u16 = 8080;
    
    HttpServer::new(move || {
        App::new()
            .service(index)
            .service(launch)
            .service(fs::Files::new("/resources", "./resources").show_files_listing())
            .service(fs::Files::new("/lib", "./lib").show_files_listing())
    })
    .bind((hostname, port))?
    .run()
    .await
}
