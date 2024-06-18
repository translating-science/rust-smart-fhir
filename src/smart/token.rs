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

use oauth2::PkceCodeVerifier;
use serde::{Deserialize, Serialize};

use std::time::{Duration, Instant};

use crate::smart::configuration::SmartConfiguration;
use crate::state::State;

#[derive(Clone)]
// Represents a Bearer token that can be used to access FHIR APIs.
pub struct Token {
    // The access token issued by the authorization server.
    pub access_token: String,

    // Scope of access authorized.
    // Note that this can be different from the scopes requested by the app.
    pub scopes: Vec<String>,

    // The point when the token expires,
    // after which the token SHALL NOT be accepted by the resource server.
    pub expires_at: Instant,

    // Token that can be used to obtain a new access token, using the same or a
    // subset of the original authorization grants
    pub refresh_token: Option<String>,

    // Authenticated user identity and user details, if requested.
    pub id_token: Option<String>
}

// NOTE: code_verifier is a secret and should not be printed
// As such, we do not support debug on this struct
#[derive(Serialize)]
struct TokenRequest {
    grant_type: String,
    code: String,
    redirect_uri: String,
    code_verifier: String
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    expires_in: u64,
    scope: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    #[allow(dead_code)]
    authorization_details: Option<String>
}

impl Token {

    fn from_response(response: TokenResponse) -> Token {
	Token {
	    access_token: response.access_token,
	    scopes: response.scope.split(" ").map(str::to_string).collect(),
	    expires_at: Instant::now() + Duration::from_secs(response.expires_in),
	    refresh_token: response.refresh_token,
	    id_token: response.id_token,
	}	    
    }

    // Requests a token from the token endpoint of a SMART-on-FHIR server.
    //
    // This method exchanges a code for a token by making a HTTP POST to the
    // token endpoint of a SMART-on-FHIR server, as documented
    // [here](https://build.fhir.org/ig/HL7/smart-app-launch/app-launch.html#obtain-access-token).
    // We are implementing a [symmetric private](https://build.fhir.org/ig/HL7/smart-app-launch/client-confidential-symmetric.html)
    // exchange.
    //
    // # Arguments
    // * `smart_configuration` The SMART configuration for the server we are requesting
    //   a token from.
    // * `code` The code received from the authorization server.
    // * `verifier` The PKCE verifier that we are exchanging.
    // * `data` The application state.
    pub async fn post(smart_configuration: &SmartConfiguration,
		      code: &String,
		      verifier: &PkceCodeVerifier,
		      data: &State) -> Result<Token, reqwest::Error> {

	// NOTE: verifier.secret is a secret and should not be printed
	let request_arguments = TokenRequest {
	    grant_type: String::from("authorization_code"),
	    code: code.clone(),
	    redirect_uri: data.callback(),
	    code_verifier: verifier.secret().clone(),
	};

	let request = data.reqwest_client.post(&smart_configuration.token_endpoint)
	    .form(&request_arguments)
	    .header("Authorization", format!("Basic {}", data.base64_secret()))
	    .send()
	    .await;
	
	match request {
	    Ok(request) => {
		let response = request.json::<TokenResponse>().await;
		
		match response {
		    Ok(response) => {
			// marshall token response
			Ok(Token::from_response(response))
		    }
		    Err(e) => Err(e)
		}
	    }
	    Err(e) => Err(e)
	}
    }       
}
