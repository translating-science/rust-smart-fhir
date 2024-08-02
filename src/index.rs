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

use actix_web::{get, web, HttpResponse, Result};
use fhir_sdk::client::Client as FhirClient;
use fhir_sdk::client::{Error, FhirR4B, SearchParameters};
use fhir_sdk::r4b::resources::{Observation, ObservationComponentValue, ObservationValue, Patient};
use fhir_sdk::{Date, TryStreamExt};
use log::error;
use maud::{html, Markup, DOCTYPE};

use crate::smart::token::ShareableToken;
use crate::state::State;

use futures::join;

// Fetches a patient resource.
//
// Fetches the [patient](http://hl7.org/fhir/R4B/patient.html) resource corresponding
// to a specific patient ID.
//
// Equivalent to:
//
// ```
// GET [base]/Patient?id=[patient_id]
// ```
//
// # Arguments
// * `client` The FHIR client to use.
// * `patient_id` The patient ID to fetch.
async fn fetch_patient(
    client: &FhirClient<FhirR4B>,
    patient_id: &str,
) -> Result<Option<Patient>, Error> {
    client.read::<Patient>(patient_id).await
}

// Fetches all observations for a specific code for a specific patient.
//
// Fetches all [observation](http://hl7.org/fhir/R4B/observation.html) resources corresponding
// to a specific patient ID, and where a specific code was observed.
//
// Equivalent to:
//
// ```
// GET [base]/Observation?subject=Patient/[patient_id]&code=[loinc]
// ```
//
// # Arguments
// * `client` The FHIR client to use.
// * `patient_id` The patient ID to fetch.
// * `loinc` The LOINC code to search for.
async fn fetch_observations(
    client: &FhirClient<FhirR4B>,
    patient_id: &str,
    loinc: &str,
) -> Result<Vec<Observation>, Error> {
    client
        .search(
            SearchParameters::empty()
                .and_raw("code", loinc)
                .and_raw("subject", format!("Patient/{patient_id}")),
        )
        .try_collect()
        .await
}

// Extracts the observed value for an observation from a query.
//
// Handles observations with [quantity](http://hl7.org/fhir/R4B/datatypes.html#Quantity)
// types. If at least one observation with a quantity type _and_ both value and unit is
// available, returns a string concatenting the value and unit. If no observations are
// found, an empty option is returned.
//
// If the query returned multiple valid Observation resources, we select one of the results.
// We do not use any specific logic to choose what to return.
//
// # Arguments
// * `search_query` The result of a query searching for observations.
fn extract_observation(search_query: Result<Vec<Observation>, Error>) -> Option<String> {
    match search_query {
        Ok(observations) => {
            // TODO: rewrite loop
            // right now, we are looping over all elements, extracting the measurement if
            // it exists, appending that into a vec, and then returning the first entry in
            // the vec. it would be more efficient to either loop until we find the first
            // valid entry, or to have smarter logic for selecting an entry to return (e.g.,
            // sort and return latest entry)
            let mut values: Vec<String> = Vec::new();

            for observation in observations {
                if let Some(ObservationValue::Quantity(quantity)) = &observation.value {
                    if let (Some(value), Some(unit)) = (&quantity.value, &quantity.unit) {
                        values.push(format!("{value} {unit}"));
                    }
                }
            }

            values.pop()
        }
        Err(e) => {
            error!("Fetching observation failed with error: {:?}", e);
            None
        }
    }
}

// Extracts the observed value for a specific component from a multi-component observation query.
//
// Handles observations that bundle multiple measurement components together. For example,
// observations corresponding to the blood pressure [LOINC 55284-4](https://loinc.org/55284-4)
// code SHOULD contain separate observations of systolic ([LOINC 8480-6](https://loinc.org/8480-6)
// and diastolic ([LOINC 8462-4](https://loinc.org/8462-4)) pressure measurements, nested under
// the [Observation.component](http://hl7.org/fhir/R4B/observation-definitions.html#Observation.component)
// field.
//
// Otherwise, behaves akin to `extract_observation`.
//
// # Arguments
// * `search_query` The result of a query searching for observations.
// * `code` The code to use to filter observation components. Should be provided without
//   the LOINC prefix; e.g., if filtering on [LOINC 8462-4](https://loinc.org/8462-4), provide
//   "8462-4", instead of "http://loinc.org|8462-4".
fn extract_observation_component(
    search_query: &Result<Vec<Observation>, Error>,
    code: String,
) -> Option<String> {
    match search_query {
        Ok(observations) => {
            // TODO: rewrite loop
            //
            // see note in `extract_observation`
            let mut values: Vec<String> = Vec::new();

            for observation in observations {
                for component in observation.component.iter().flatten() {
                    for coding in component.code.coding.iter().flatten() {
                        if Some(code.clone()) == coding.code {
                            if let Some(ObservationComponentValue::Quantity(quantity)) =
                                &component.value
                            {
                                if let (Some(value), Some(unit)) = (&quantity.value, &quantity.unit)
                                {
                                    values.push(format!("{value} {unit}"));
                                }
                            }
                        }
                    }
                }
            }

            values.pop()
        }
        Err(e) => {
            error!("Fetching observation failed with error: {:?}", e);
            None
        }
    }
}

/**
 * FHIR app: patient data visualizer
 * ---------------------------------
 * Once we have completed the launch process and received a valid FHIR API token, our
 * `/callback` endpoint will redirect the user here. This endpoint will use the FHIR API
 * token corresponding to a patient ID (encoded in the path) to display a simple summary
 * about the patient we have selected. This summary shows:
 *
 * - Patient name and birthdate, taken from the [FHIR patient resource](http://hl7.org/fhir/R4B/patient.html)
 * - Several measurements, taken from [FHIR observations](http://hl7.org/fhir/R4B/observation.html) associated with the patient. These measurements may not be available for all patients.
 *   - A blood pressure measurement, using the combined measurement code [LOINC 55284-4](https://loinc.org/55284-4).
 *     Systolic/diastolic measurements are broken out by processing the individual
 *     [observation components](http://hl7.org/fhir/R4B/observation-definitions.html#Observation.component).
 *   - Height, using the code [LOINC 8302-2](https://loinc.org/8302-2).
 *   - LDL, using the code [LOINC 2089-1](https://loinc.org/2089-1).
 *   - HDL, using the code [LOINC 2085-9](https://loinc.org/2085-9).
 */
#[get("/{patient_id}/index.html")]
pub async fn index(data: web::Data<State>, patient_id: web::Path<String>) -> HttpResponse {
    if let Some(token) = data.get_token(&patient_id) {
        let (patient_id, iss) = token.patient_and_iss();

        match ShareableToken::build_client(data.reqwest_client.clone(), iss.clone(), token.clone())
            .await
        {
            Ok(client) => {
                // fetch the core patient data
                let patient_request = fetch_patient(&client, &patient_id);

                // loinc codes - these need to have a lifetime that persists until the `join!`
                let bp_loinc = String::from("http://loinc.org|55284-4");
                let height_loinc = String::from("http://loinc.org|8302-2");
                let ldl_loinc = String::from("http://loinc.org|2089-1");
                let hdl_loinc = String::from("http://loinc.org|2085-9");

                // fetch observations from FHIR server
                // TODO:
                // - we are currently collecting all observations. this is fine for test data,
                //   but is not the ideal way to handle the data.
                // - the LDL code seems to not fetch any data from the SMART test server...
                //   are we using an incorrect code? needs more exploration...
                let blood_pressure_request = fetch_observations(&client, &patient_id, &bp_loinc);
                let height_request = fetch_observations(&client, &patient_id, &height_loinc);
                let ldl_request = fetch_observations(&client, &patient_id, &ldl_loinc);
                let hdl_request = fetch_observations(&client, &patient_id, &hdl_loinc);
                let (patient, blood_pressure, height, ldl, hdl) = join!(
                    patient_request,
                    blood_pressure_request,
                    height_request,
                    ldl_request,
                    hdl_request
                );

                // if we have received a valid patient resource, then render the page.
                // we are more lenient with error checking for the observations, as we do not
                // expect to find observations for all codes for all patients.
                match patient {
                    Ok(Some(patient)) => HttpResponse::Ok()
                        .body(render_page(patient, blood_pressure, height, ldl, hdl).into_string()),
                    Ok(None) => HttpResponse::NotFound()
                        .body(format!("No search results found for {}", patient_id)),
                    Err(e) => HttpResponse::InternalServerError()
                        .body(format!("Searching for patient failed with error: {:?}", e)),
                }
            }
            Err(e) => HttpResponse::InternalServerError()
                .body(format!("Failed to create FHIR client. Error was: {:?}", e)),
        }
    } else {
        HttpResponse::InternalServerError().body(format!("Failed to find token for {patient_id}."))
    }
}

// Formats a FHIR date for display.
//
// # Arguments
// * `date` The date to display.
fn display_date(date: &Date) -> String {
    // TODO: move this function into a centralized location
    match date {
        Date::Year(year) => format!("{}", year),
        Date::YearMonth(year, month) => format!("{month} {year}"),
        Date::Date(date) => format!("{} {}, {}", date.month(), date.day(), date.year()),
    }
}

// Generates the HTML for the queried patient and observations.
#[rustfmt::skip::macros(html)]
fn render_page(
    patient: Patient,
    blood_pressure: Result<Vec<Observation>, Error>,
    height: Result<Vec<Observation>, Error>,
    ldl: Result<Vec<Observation>, Error>,
    hdl: Result<Vec<Observation>, Error>,
) -> Markup {
    html! {
	(DOCTYPE);
	html lang="en" {
            head {
		title {
		    "Example SMART-on-FHIR app"
		}
            }
            body {
		div #holder {
		    h1 {
			"Example SMART-on-FHIR app"
		    }
		    section #patient {
			h2 {
			    "Patient resource"
			}
			table {
			    tbody {
				@if !patient.name.is_empty() {
				    @if let Some(name) = &patient.name[0] {
					tr {
					    th {
						"First name:"
					    }
					    td #fname {
						@if !name.given.is_empty() {
						    @if let Some(given_name) = &name.given[0] {
							(given_name)
						    }
						}
					    }
					}
					tr {
					    th {
						"Last name:"
					    }
					    td #lname {
						@if let Some(family_name) = &name.family {
						    (family_name)
						}
					    }
					}
				    }
				}
			    }
			    @if let Some(gender) = &patient.gender {
				tr {
				    th {
					"Gender:"
				    }
				    td #gender {
					(gender)
				    }
				}
			    }
			    @if let Some(birth_date) = &patient.birth_date {
				tr {
				    th {
					"Date of birth:"
				    }
				    td #birthdate {
					(display_date(birth_date))
				    }
				}
			    }
			}
		    }
		    section #observation {
			h2 {
			    "Observation resource"
			}
			table {
			    tbody {
				@if let Some(height) = extract_observation(height) {
				    tr {
					th {
					    "Height:"
					}
					td #height {
					    (height)
					}
				    }
				}
				@if let Some(systolic_blood_pressure) = extract_observation_component(&blood_pressure, String::from("8480-6")) {
				    tr {
					th {
					    "Systolic blood pressure:"
					}
					td #systolicbp {
					    (systolic_blood_pressure)
					}
				    }
				}
				@if let Some(diastolic_blood_pressure) = extract_observation_component(&blood_pressure, String::from("8462-4")) {
				    tr {
					th {
					    "Diastolic blood pressure:"
					}
					td #disatolicbp {
					    (diastolic_blood_pressure)
					}
				    }
				}
				@if let Some(ldl) = extract_observation(ldl) {
				    tr {
					th {
					    "LDL:"
					}
					td #ldl {
					    (ldl)
					}
				    }
				}
				@if let Some(hdl) = extract_observation(hdl) {
				    tr {
					th {
					    "HDL:"
					}
					td #hdl {
					    (hdl)
					}
				    }
				}
			    }
			}
		    }
		}
            }
	}
    }
}
