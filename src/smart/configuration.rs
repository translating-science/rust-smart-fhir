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

use reqwest::Client;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub struct Endpoint {
    url: String,
    capabilities: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub struct SmartConfiguration {
    // CONDITIONAL, String conveying this system’s OpenID Connect Issuer URL.
    // Required if the server’s capabilities include sso-openid-connect; otherwise, omitted.
    pub issuer: Option<String>,

    // CONDITIONAL, String conveying this system’s JSON Web Key Set URL.
    // Required if the server’s capabilities include sso-openid-connect; otherwise, optional.
    pub jwks_url: Option<String>,

    // CONDITIONAL, URL to the OAuth2 authorization endpoint.
    // Required if server supports the launch-ehr or launch-standalone capability; otherwise, optional.
    pub authorization_endpoint: Option<String>,

    // REQUIRED, Array of grant types supported at the token endpoint.
    // The options are “authorization_code” (when SMART App Launch is supported) and
    // “client_credentials” (when SMART Backend Services is supported).
    pub grant_types_supported: Vec<String>,

    // REQUIRED, URL to the OAuth2 token endpoint.
    pub token_endpoint: String,

    // OPTIONAL, array of client authentication methods supported by the token endpoint.
    // The options are “client_secret_post”, “client_secret_basic”, and “private_key_jwt”.
    pub token_endpoint_auth_methods_supported: Vec<String>,

    // OPTIONAL, If available, URL to the OAuth2 dynamic registration endpoint for this FHIR server.
    pub registration_endpoint: Option<String>,

    // OPTIONAL, DEPRECATED, URL to the EHR’s app state endpoint.
    // Deprecated; use associated_endpoints with the smart-app-state capability instead.
    #[deprecated]
    pub smart_app_state_endpoint: Option<String>,

    // OPTIONAL, Array of objects for endpoints that share the same authorization mechanism as this FHIR endpoint, each with a “url” and “capabilities” array.
    // This property is deemed experimental.
    pub associated_endpoints: Option<Endpoint>,

    // RECOMMENDED, URL for a Brand Bundle.
    pub user_access_brand_bundle: Option<String>,

    // RECOMMENDED, Identifier for the primary entry in a Brand Bundle.
    pub user_access_brand_identifier: Option<String>,

    // RECOMMENDED, Array of scopes a client may request.
    // The server SHALL support all scopes listed here; additional scopes MAY be supported (so clients should not consider this an exhaustive list).
    pub scopes_supported: Vec<String>,

    // RECOMMENDED, Array of OAuth2 response_type values that are supported.
    pub response_types_supported: Vec<String>,

    // RECOMMENDED, URL where an end-user can view which applications currently have access to data and can make adjustments to these access rights.
    pub management_endpoint: Option<String>,

    // RECOMMENDED, URL to a server’s introspection endpoint that can be used to validate a token.
    pub introspection_endpoint: Option<String>,

    // RECOMMENDED, URL to a server’s revoke endpoint that can be used to revoke a token.
    pub revocation_endpoint: Option<String>,

    // REQUIRED, Array of strings representing SMART capabilities (e.g., sso-openid-connect or launch-standalone) that the server supports.
    pub capabilities: Vec<String>,

    // REQUIRED, Array of PKCE code challenge methods supported. The S256 method SHALL be included in this list, and the plain method SHALL NOT be included in this list.
    pub code_challenge_methods_supported: Vec<String>,
}

impl SmartConfiguration {
    pub async fn get(
        base_url: &String,
        client: &Client,
    ) -> Result<SmartConfiguration, reqwest::Error> {
        let request = client
            .get(format!("{}/.well-known/smart-configuration", base_url))
            .header("Accept", "application/json")
            .build();

        match request {
            Ok(req) => {
                let smart_configuration = client.execute(req).await;

                match smart_configuration {
                    Ok(smart_configuration) => {
                        smart_configuration.json::<SmartConfiguration>().await
                    }
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }
}
