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

use actix_web::{get, web, HttpResponse};
use log::{debug, error};
use oauth2::PkceCodeChallenge;
use serde::Deserialize;
use url::Url;
use url_builder::URLBuilder;
use uuid::Uuid;

use crate::smart::configuration::SmartConfiguration;
use crate::state::State;

#[derive(Deserialize)]
struct LaunchQuery {
    // URL of the FHIR server
    iss: String,
    // Unique launch ID parameter received from the launching EHR
    launch: String,
}

/**
 * SMART-on-FHIR EHR launch sequence: step 1 (launching)
 * -----------------------------------------------------
 * When an EHR launches a SMART-on-FHIR app, it will call the `launch` endpoint
 * and provide two arguments:
 *
 * - `iss`: This is the base URL of the FHIR server of the EHR.
 * - `launch`: This is a unique ID for the SMART-on-FHIR app launch from this
 *   FHIR instance.
 *
 * The app should then call to the FHIR server's `.well-known/smart-configuration`
 * endpoint, which will provide metadata about the OAuth endpoints needed for
 * SMART authorization against this EHR. Documentation on this endpoint can be
 * found [here](http://hl7.org/fhir/smart-app-launch/conformance.html#example-request).
 *
 * Once the OAuth endpoints have been discovered, the `launch` endpoint should proceed
 * to request an authorization code from the SMART OAuth endpoints. This is done by causing
 * the browser to navigate to the EHR's authorization URL, with a specific set of
 * [parameters](http://hl7.org/fhir/smart-app-launch/app-launch.html#obtain-authorization-code),
 * including the redirect URL.
 *
 * The EHR will then redirect to the redirect URL ("/callback", in our case), which
 * continues the authorization flow by requesting a token.
 */
#[get("/launch")]
pub async fn launch(data: web::Data<State>, query: web::Query<LaunchQuery>) -> HttpResponse {
    // Get the .well-known/smart-configuration from the FHIR server.
    let smart_configuration = SmartConfiguration::get(&query.iss, &data.reqwest_client).await;

    match smart_configuration {
        Ok(smart_configuration) => {
            debug!(
                "Successfully retrieved SMART configuration from issuer {}",
                query.iss
            );

            if let Some(authorization_endpoint) = &smart_configuration.authorization_endpoint {
                let auth_url = Url::parse(authorization_endpoint);

                match auth_url {
                    Ok(auth_url) => {
                        // Create a PKCE S256 code verifier / challenge pair.
                        let (pkce_challenge, pkce_verifier) =
                            PkceCodeChallenge::new_random_sha256();

                        // Create a UUID to use as state.
                        let state = Uuid::new_v4();

                        // Insert smart configuration and issuer for state
                        data.put_iss_and_config(&state, &query.iss, &smart_configuration);

                        // Insert PKCE into app state for use from callback endpoint
                        data.put_pkce(&state, pkce_challenge.clone(), pkce_verifier);

                        debug!(
                            "Redirecting launch from issuer {} with state {} to {}",
                            query.iss, state, auth_url
                        );

                        // Create a HTTP response that redirects the web browser to the EHR authorization endpoint.
                        // This is described in the
                        // [SMART-on-FHIR docs](https://build.fhir.org/ig/HL7/smart-app-launch/app-launch.html#obtain-authorization-code).
                        //
                        // To trigger the redirect, we are using a
                        // ["303 See Other"](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/303)
                        // HTTP response, and are setting the ["Location"
                        // header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Location) to the authorization
                        // endpoint on the EHR.
                        HttpResponse::SeeOther()
                            .insert_header((
                                actix_web::http::header::LOCATION,
                                authorize_url(
                                    data,
                                    &auth_url,
                                    &query.iss,
                                    &query.launch,
                                    pkce_challenge.as_str(),
                                    &state,
                                ),
                            ))
                            .finish()
                    }
                    Err(e) => {
                        error!("Failed to parse authorization server URL {authorization_endpoint} due to error {e}");
                        HttpResponse::InternalServerError().body(format!(
			    "Failed to parse authorization URL provided by EHR: {}\nError was: {:?}",
			    authorization_endpoint, e
			))
                    }
                }
            } else {
                let err = format!(
                    "EHR {} does not provide an authorization endpoint.",
                    &query.iss
                );
                error!("{err}");
                HttpResponse::NotImplemented().body(err)
            }
        }
        Err(e) => {
            error!(
                "Fetching SMART configuration from EHR {} failed due to {:?}",
                query.iss, e
            );
            HttpResponse::InternalServerError().body(format!(
                "Failed to parse SMART configuration provided by EHR {}.",
                &query.iss
            ))
        }
    }
}

fn authorize_url(
    data: web::Data<State>,
    base_url: &Url,
    iss: &str,
    launch_id: &str,
    code_challenge: &str,
    state: &Uuid,
) -> String {
    let desired_scopes = [
        "patient/Patient.read",
        "patient/Observation.read",
        "launch",
        "launch/patient",
        "online_access",
        "openid",
        "profile",
    ];

    let mut ub = URLBuilder::new();

    ub.set_protocol(base_url.scheme())
        .set_host(base_url.host_str().unwrap_or(""))
        .add_route(base_url.path().trim_matches('/'))
        .add_param("response_type", "code")
        .add_param("client_id", &data.client_id)
        .add_param("redirect_uri", &data.callback())
        .add_param("launch", launch_id)
        .add_param("state", &state.to_string())
        .add_param("aud", iss)
        .add_param("code_challenge", code_challenge)
        .add_param("code_challenge_method", "S256")
        .add_param("scope", &desired_scopes.join("+"));

    ub.build()
}
