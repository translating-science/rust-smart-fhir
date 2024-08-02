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

use base64::prelude::{Engine, BASE64_STANDARD};
use oauth2::{PkceCodeChallenge, PkceCodeVerifier};
use reqwest::Client;
use uuid::Uuid;

use crate::smart::configuration::SmartConfiguration;
use crate::smart::token::{ShareableToken, Token};

use std::collections::HashMap;
use std::sync::Mutex;

pub struct State {
    pub app_domain: String,
    pub client_id: String,
    pub client_secret: String,
    pub reqwest_client: Client,

    pkce: Mutex<HashMap<Uuid, (PkceCodeChallenge, PkceCodeVerifier)>>,
    smart_configurations: Mutex<HashMap<String, SmartConfiguration>>,
    iss: Mutex<HashMap<Uuid, String>>,
    tokens: Mutex<HashMap<String, ShareableToken>>,
}

impl State {
    pub fn new(app_domain: String, client_id: String, client_secret: String) -> State {
        State {
            app_domain,
            client_id,
            client_secret,
            reqwest_client: Client::new(),
            pkce: Mutex::new(HashMap::new()),
            smart_configurations: Mutex::new(HashMap::new()),
            iss: Mutex::new(HashMap::new()),
            tokens: Mutex::new(HashMap::new()),
        }
    }

    /// Provides a secret usable with the SMART-on-FHIR symmetric authorization flow.
    ///
    /// Base64 encodes "client_id:client_secret", as described in the SMART-on-FHIR
    /// [docs](https://build.fhir.org/ig/HL7/smart-app-launch/client-confidential-symmetric.html).
    pub fn base64_secret(&self) -> String {
        BASE64_STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret))
    }

    // Generates the callback URL for this app.
    pub fn callback(&self) -> String {
        format!("{}/callback", self.app_domain)
    }

    // Adds the issuer and SMART configuration into the state store.
    //
    // At the start of a SMART launch, we collect a SMART configuration from the
    // SMART-on-FHIR server that issued the launch. This method stores the
    // issuer, keyed by the launch UUID (`state`), and the configuration (keyed
    // by the issuer URL).
    //
    // # Arguments
    // * `state` The UUID for the launch.
    // * `iss` The URL of the server that issued the launch.
    // * `config` The SMART Configuration for the server.
    pub fn put_iss_and_config(&self, state: &Uuid, iss: &str, config: &SmartConfiguration) {
        self.put_iss(state, iss);
        self.put_config(iss, config);
    }

    fn put_iss(&self, state: &Uuid, iss: &str) {
        let mut map = self.iss.lock().unwrap();
        map.insert(*state, iss.to_string());
    }

    fn get_iss(&self, state: &Uuid) -> Option<String> {
        let mut map = self.iss.lock().unwrap();
        map.remove(state)
    }

    fn put_config(&self, iss: &str, config: &SmartConfiguration) {
        let mut map = self.smart_configurations.lock().unwrap();
        map.insert(iss.to_string(), config.clone());
    }

    // Gets the issuer and SMART configuration from the state store.
    //
    // Retrieves the issuer URL and SMART configuration associated with a launch
    // UUID (`state`). This method can only be called once for a given
    // `state` UUID; calling it more than once will lead to a `None` option
    // being returned.
    //
    // # Arguments
    // * `state` The UUID for the launch.
    pub fn get_iss_and_config(&self, state: &Uuid) -> Option<(String, SmartConfiguration)> {
        let iss = self.get_iss(state);
        if let Some(iss) = iss {
            let map = self.smart_configurations.lock().unwrap();
            map.get(&iss).map(|config| (iss, config.clone()))
        } else {
            None
        }
    }

    // Adds the PKCE challenge/verifier pair for a launch to the state store.
    //
    // The SMART-on-FHIR confidential launch flow depends on a [PKCE
    // exchange](https://oauth.net/2/pkce/) to protect against a number of forgery
    // and injection attacks. We generate a PKCE challenge / verifier pair during
    // launch; this method stores them so that the callback endpoint can access
    // them during the token exchange.
    //
    // # Arguments
    // * `state` The UUID for the launch.
    // * `challenge` The PKCE challenge code.
    // * `verifier` The PKCE verifier code.
    pub fn put_pkce(&self, state: &Uuid, challenge: PkceCodeChallenge, verifier: PkceCodeVerifier) {
        let mut map = self.pkce.lock().unwrap();
        map.insert(*state, (challenge, verifier));
    }

    // Gets the PKCE challenge/verifier pair for a launch from the state store.
    //
    // This function retrieves the PKCE challenge / verifier codes corresponding to
    // a launch UUID (`state`). It can be called once per state UUID; calling it
    // further will lead to a `None` option being returned.
    //
    // # Arguments
    // * `state` The UUID for the launch.
    pub fn get_pkce(&self, state: &Uuid) -> Option<(PkceCodeChallenge, PkceCodeVerifier)> {
        let mut map = self.pkce.lock().unwrap();
        map.remove(state)
    }

    // Puts a FHIR Bearer token into the state store.
    //
    // # Arguments
    // * `iss` The URL of the issuer of the token.
    // * `token` The Bearer token.
    pub fn put_token(&self, token: Token) {
        let mut map = self.tokens.lock().unwrap();
        map.insert(token.patient.clone(), ShareableToken::new(token));
    }

    // Gets an issuer URL and FHIR Bearer token from the state store.
    //
    // This function can be called multiple times.
    //
    // # Arguments
    // * `patient_id` The patient ID to return a token for.
    pub fn get_token(&self, patient_id: &str) -> Option<ShareableToken> {
        // TODO: here we assume that patient_ids are globally unique
        // this is a faulty assumption that we should fix at a later date.

        let map = self.tokens.lock().unwrap();
        map.get(patient_id).cloned()
    }
}
