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

use fhir_sdk::client::Client as FhirClient;
use fhir_sdk::client::{Error, FhirR4B, LoginManager};
use fhir_sdk::header::InvalidHeaderValue;
use fhir_sdk::{HeaderValue, HttpClient};
use oauth2::PkceCodeVerifier;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::smart::configuration::SmartConfiguration;
use crate::state::State;

// Represents a Bearer token that can be used to access FHIR APIs.
pub struct Token {
    // The SMART Configuration for the FHIR server this token was
    // requested from. Used for refreshing the token.
    smart_configuration: SmartConfiguration,

    // The BASE64 secret for the app. Used for refreshing the token.
    base64_secret: String,

    // Reqwest client, used for refreshing the token.
    reqwest_client: ReqwestClient,

    // the core token fields
    token: TokenContents,

    // The ID for the selected patient, requested via `launch/patient` scope.
    pub patient: String,

    // The URL that issued this Token.
    iss: String,
}

#[derive(Clone)]
struct TokenContents {
    // The access token issued by the authorization server.
    access_token: String,

    // Scope of access authorized.
    // Note that this can be different from the scopes requested by the app.
    #[allow(dead_code)]
    scopes: Vec<String>,

    // The point when the token expires,
    // after which the token SHALL NOT be accepted by the resource server.
    expires_at: Instant,

    // Token that can be used to obtain a new access token, using the same or a
    // subset of the original authorization grants
    refresh_token: Option<String>,

    // Authenticated user identity and user details, if requested.
    #[allow(dead_code)]
    id_token: Option<String>,
}

// NOTE: code_verifier is a secret and should not be printed
// As such, we do not support debug on this struct
#[derive(Serialize)]
struct TokenRequest {
    grant_type: String,
    code: String,
    redirect_uri: String,
    code_verifier: String,
}

// NOTE: refresh_token is a secret and should not be printed
// As such, we do not support debug on this struct
#[derive(Serialize)]
struct TokenRefreshRequest {
    grant_type: String,
    refresh_token: String,
    // We omit the `scopes` parameter, as to request the same scopes as were in the
    // original token.
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
    patient: String,
    #[allow(dead_code)]
    authorization_details: Option<String>,
}

#[derive(Clone)]
pub struct ShareableToken {
    token: Arc<RwLock<Token>>,
}

impl ShareableToken {
    pub fn new(token: Token) -> ShareableToken {
        ShareableToken {
            token: Arc::new(RwLock::new(token)),
        }
    }

    pub fn patient_and_iss(&self) -> (String, String) {
        let token = self.token.read().unwrap();
        (token.patient.clone(), token.iss.clone())
    }

    // Builds a FHIR API client.
    //
    // Configures a FHIR API client that targets the FHIR API that issued our
    // token, with the bearer token set in the authorization header.
    //
    // # Arguments
    // * `client` The Reqwest client that we will use for sending HTTP requests.
    // * `iss` The URL of the FHIR server that issued our token.
    // * `token` The token to use for authorization.
    pub async fn build_client(
        client: ReqwestClient,
        iss: String,
        token: ShareableToken,
    ) -> Result<FhirClient<FhirR4B>, Error> {
        {
            // TODO: ideally we should preserve the client?
            FhirClient::<FhirR4B>::builder()
                .client(client)
                .base_url(iss.parse().unwrap())
                .auth_callback(token)
                .build()
        }
    }
}

// Extends trait from fhir_sdk, used to create authorization headers for
// FHIR Client requests.
impl LoginManager for ShareableToken {
    type Error = InvalidHeaderValue;

    async fn authenticate(
        &mut self,
        _client: HttpClient,
    ) -> Result<HeaderValue, <ShareableToken as LoginManager>::Error> {
        // Here, we read lock the token to see if it is still valid, or whether
        // it needs a refresh.
        let token_needing_refresh = {
            let token = self.token.read().unwrap();

            if token.needs_refresh() {
                Some((
                    token.token.clone(),
                    token.smart_configuration.clone(),
                    token.base64_secret.clone(),
                    token.reqwest_client.clone(),
                ))
            } else {
                None
            }
        };

        // If the token needs to be refreshed, we issue the refresh API call.
        //
        // If the API call succeeds and we get a refreshed token, we write lock
        // the token and insert the updated token.
        //
        // TODO: ideally the read / write pattern here would be a single transaction.
        // However, we cannot hold a std::sync::RwLock across an async function call,
        // hence the lock / unlock / relock pattern. This could arguably lead to errors.
        if let Some((inner_token, smart_configuration, base64_secret, reqwest_client)) =
            token_needing_refresh
        {
            let refreshed_token = inner_token
                .refresh(&reqwest_client, &smart_configuration, &base64_secret)
                .await;

            if let Ok(refreshed_token) = refreshed_token {
                let mut token = self.token.write().unwrap();
                token.refresh_token(refreshed_token)
            }
        }

        {
            let token = self.token.read().unwrap();
            token.auth_header()
        }
    }
}

impl TokenContents {
    fn split_scopes(scope: String) -> Vec<String> {
        scope.split(' ').map(str::to_string).collect()
    }

    fn expiration(expires_in: u64) -> Instant {
        Instant::now() + Duration::from_secs(expires_in)
    }

    fn from_response(response: TokenResponse) -> TokenContents {
        TokenContents {
            access_token: response.access_token,
            scopes: Self::split_scopes(response.scope),
            expires_at: Self::expiration(response.expires_in),
            refresh_token: response.refresh_token,
            id_token: response.id_token,
        }
    }

    fn has_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }

    // Refreshes the token by calling to the FHIR server's token endpoint
    // with token refresh arguments.
    //
    // Does not update in place, rather this method returns a new token.
    async fn refresh(
        &self,
        reqwest_client: &ReqwestClient,
        smart_configuration: &SmartConfiguration,
        base64_secret: &str,
    ) -> Result<TokenContents, reqwest::Error> {
        let refresh_token = self
            .refresh_token
            .as_ref()
            .expect("Tried to refresh token that was not refreshable.");

        // NOTE: the refresh token is a secret and should not be printed
        let request_arguments = TokenRefreshRequest {
            grant_type: String::from("refresh_token"),
            refresh_token: refresh_token.clone(),
        };

        let request = reqwest_client
            .post(&smart_configuration.token_endpoint)
            .form(&request_arguments)
            .header("Authorization", format!("Basic {}", base64_secret))
            .send()
            .await;

        match request {
            Ok(request) => {
                let response = request.json::<TokenResponse>().await;

                match response {
                    Ok(response) => {
                        // marshall token response
                        Ok(TokenContents::from_response(response))
                    }
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl Token {
    fn auth_header(&self) -> Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::from_str(&format!(
            "AUTHORIZATION: Bearer {}",
            self.token.access_token
        ))
    }

    fn needs_refresh(&self) -> bool {
        self.token.has_expired() && self.token.can_refresh()
    }

    fn refresh_token(&mut self, contents: TokenContents) {
        self.token = contents;
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
    pub async fn post(
        smart_configuration: &SmartConfiguration,
        code: &str,
        verifier: &PkceCodeVerifier,
        data: &State,
    ) -> Result<Token, reqwest::Error> {
        // NOTE: verifier.secret is a secret and should not be printed
        let request_arguments = TokenRequest {
            grant_type: String::from("authorization_code"),
            code: code.to_string(),
            redirect_uri: data.callback(),
            code_verifier: verifier.secret().clone(),
        };

        let request = data
            .reqwest_client
            .post(&smart_configuration.token_endpoint)
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
                        Ok(Token {
                            smart_configuration: smart_configuration.clone(),
                            base64_secret: data.base64_secret(),
                            reqwest_client: data.reqwest_client.clone(),
                            patient: response.patient.clone(),
                            iss: smart_configuration.issuer.clone().unwrap(),
                            token: TokenContents::from_response(response),
                        })
                    }
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }
}
