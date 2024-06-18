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

use actix_web::{get, Result};
use maud::{html, DOCTYPE, Markup, PreEscaped};

#[get("/index.html")]
pub async fn index() -> Result<Markup> {
    Ok(html! {
	(DOCTYPE);
	html lang="en" {
	    head {
		meta http-equiv="X-UA-Compatible" content="IE=edge" {}
		meta http-equiv="Content-Type" content="text/html; charset=utf-8" {}
		title {
		    "Example SMART-on-FHIR app"
		}
	    }
	    body {
		div #errors {}
		div #loading .spinner {
		    div .bounce1 {}
		    div .bounce2 {}
		    div .bounce3 {}
		}
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
				tr {
				    th {
					"First name:"
				    }
				    td #fname {
				    }
				}
				tr {
				    th {
					"Last name:"
				    }
				    td #lname {
				    }
				}
				tr {
				    th {
					"Gender:"
				    }
				    td #gender {
				    }
				}
				tr {
				    th {
					"Date of birth:"
				    }
				    td #birthdate {
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
				tr {
				    th {
					"Height:"
				    }
				    td #height {					
				    }
				}
				tr {
				    th {
					"Systolic blood pressure:"
				    }
				    td #systolicbp {					
				    }
				}
				tr {
				    th {
					"Diastolic blood pressure:"
				    }
				    td #disatolicbp {					
				    }
				}
				tr {
				    th {
					"LDL:"
				    }
				    td #ldl {					
				    }
				}
				tr {
				    th {
					"HDL:"
				    }
				    td #hdl {					
				    }
				}
			    }
			}
		    }
		    script src="/resources/example-smart-app.js" {}
		    script src="/lib/fhir-client-v0.1.12.js" {}
		    script src="/lib/fhir-client-cerner-additions-1.0.0.js" {}
		    script src="https://ajax.googleapis.com/ajax/libs/jquery/1.12.4/jquery.min.js" {}
		    script {
			(PreEscaped(r#"extractData().then(
        // Display Patient Demographics and Observations if extractData was success
        function(p) {
          drawVisualization(p);
        },

        // Display 'Failed to call FHIR Service' if extractData failed
        function() {
          $('#loading').hide();
          $('#errors').html('<p> Failed to call FHIR Service </p>');
        }
      );"#))
		    }
		}
	    }		
	}
    })
}
