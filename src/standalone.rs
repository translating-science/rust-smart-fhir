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

use actix_web::{get, Result as AwResult};
use maud::{html, Markup, DOCTYPE};

/**
 * SMART-on-FHIR EHR standalone launch sequence: step 0
 * ----------------------------------------------------
 * In the "standalone launch flow", a user will launch a connection to the EHR
 * through a standalone app. To do this, the user / app must provide the URL of
 * the EHR to launch against, which becomes the basis for the rest of the launch
 * flow (see notes in launch.rs).
 *
 * Here, we provide an endpoint at `/standalone.html` that serves a website that
 * allows a user to enter an EHR's FHIR URL via an HTML form. Once the user provides
 * the URL, we proceed with the SMART-on-FHIR launch sequence by making an HTTP
 * `get` request against the `/launch` endpoint while providing the FHIR URL.
 * This is briefly documented in the [FHIR documentation](https://build.fhir.org/ig/HL7/smart-app-launch/app-launch.html#launch-app-standalone-launch).
 *
 * In a real world application, instead of using a web form, we would likely
 * pre-register our app with an EHR provider who would provide us with a list of
 * EHR FHIR URLs, or a mechanism to programmatically get the FHIR URLs.
 */
#[rustfmt::skip::macros(html)]
#[get("/standalone.html")]
pub async fn standalone_launcher() -> AwResult<Markup> {
    Ok(html! {
	(DOCTYPE);
	html lang="en" {
            head {
		title {
		    "Example SMART-on-FHIR app: Standalone launch"
		}
            }
            body {
		main {
		    h1 {
			"Provide an EHR endpoint to launch against"
		    }
		    form action="/launch" method="get" {
			div {
			    label for="iss" {
				"URL of EHR endpoint: "
			    }
			    input type="url" name="iss" id="iss" required {}
			}
			div {
			    input type="submit" value="Launch" {}
			}
		    }
		}
	    }
	}
    })
}
