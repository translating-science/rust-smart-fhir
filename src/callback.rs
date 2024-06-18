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
use serde::Deserialize;
use uuid::Uuid;

use crate::smart::token::Token;
use crate::state::State;

#[allow(dead_code)]
#[derive(Deserialize)]
struct CallbackQuery {
    // The authorization code generated by the authorization server.
    // The authorization code needs to expire shortly after it is issued to mitigate the risk of leaks.
    code: String,

    // The exact state value received from the client on the authorization call.
    state: String,
}

/**
 * SMART-on-FHIR EHR launch sequence: step 2 (acquiring token)
 * -----------------------------------------------------------
 * In step 1, our /launch endpoint talked to the FHIR server to acquire it's configuration
 * and to request an authorization code. When requesting an authorization code, we passed
 * a redirect URL to the FHIR server. That redirect URL represents this endpoint.
 *
 * This endpoint completes the next step of the launch sequence: exchanging an authorization
 * code for a token. Since we only store the token in a secure backend, our  app is considered
 * a ["confidential"](https://build.fhir.org/ig/HL7/smart-app-launch/app-launch.html#support-for-public-and-confidential-apps)
 * app by OIDC / SMART-on-FHIR. In this example, we are using a symmetric authentication flow
 * where we have pre-registered a client secret with the FHIR server.
 *
 * To exchange the code for a token, we need to POST to the FHIR server's token endpoint as
 * described [here](https://build.fhir.org/ig/HL7/smart-app-launch/app-launch.html#step-5-access-token).
 * Once we have the token, we can call against the core FHIR APIs.
 */
#[get("/callback")]
pub async fn callback(data: web::Data<State>, query: web::Query<CallbackQuery>) -> HttpResponse {

    // parse state value to get transaction uuid
    match Uuid::parse_str(&query.state) {
	Ok(state) => {
	    // get PKCE challenge / verifier pair for this transaction
	    match data.get_pkce(&state) {
		Some((_challenge, verifier)) => {

		    // get smart configuration for this transaction
		    let configuration = data.get_iss_and_config(&state);

		    match configuration {
			Some((iss, smart_configuration)) => {
			    // call to the FHIR server to request a token
			    let token = Token::post(&smart_configuration,
						    &query.code,
						    &verifier,
						    &data).await;

			    match token {
				Ok(token) => {
				    // if we've received a token, store it
				    data.put_token(&iss, token);

				    // TODO: update index.html to use token and change this
				    // response to redirect to index.html
				    HttpResponse::Ok()
					.body("Successfully exchanged token.")
				}
				Err(_) => {
				    HttpResponse::Forbidden()
					.body("Failed to exchange token.")
				}
			    }
			}
			None => {
			    HttpResponse::InternalServerError()
				.body("Could not find SMART configuration for transaction.")
			}
		    }
		}
		None => {
		    HttpResponse::BadRequest()
			.body("Received unknown state parameter from EHR.")
		}
	    }
	}
	Err(_) => {
	    HttpResponse::BadRequest()
		.body("Failed to parse state parameter provided by EHR.")
	}
    }
}
