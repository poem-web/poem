## Merging API Specifications

To merge API specifications for multiple services and expose them on a single page, follow these steps:

1. Generate OpenAPI specifications for each service.
2. Create a function to merge the specifications. ...
3. Integrate the merged specification when creating the OpenApiService.
4. Test the merged API specification to ensure it works as expected.

Example code snippet:

```rust
// Merge OpenAPI specifications
let merged_spec = merge_openapi_specs(auth_spec, test_spec);

// Create an OpenApiService for the merged specification
let api_service = OpenApiService::new_with_spec(merged_spec, "Merged API", version).server(api_doc_url_info);

___________________________________________________________________________


## Merging API Specifications [Explained]:

If you have two API services and wish to merge their OpenAPI specifications to be accessed on a single page, follow these steps:

1.Generate OpenAPI Specifications:
   
    Ensure that you have the OpenAPI specifications for each service. You can use tools like Swagger or OpenAPI Generator to automatically generate these specifications from your API code.

2.Merge Specifications:
    
    Create a function to merge the OpenAPI specifications. Below is an example code snippet. Customize the function based on the structure of your OpenAPI specifications. Ensure that you handle conflicts appropriately.:


lang:'Rust'

use openapiv3::OpenAPI;

fn merge_openapi_specs(auth_spec: OpenAPI, test_spec: OpenAPI) -> OpenAPI {
    let mut merged_spec = auth_spec.clone(); // Start with one of the specs

    // Merge paths
    if let Some(test_paths) = test_spec.paths {
        if let Some(merged_paths) = &mut merged_spec.paths {
            merged_paths.extend(test_paths);
        } else {
            merged_spec.paths = Some(test_paths);
        }
    }

    // Merge components, etc.

    merged_spec
}


3. Integrate Merged Specification:
    
    Use the merged specification when creating the OpenApiService. Update your application code as following example:

lang:'Rust'

use poem_openapi::{OpenApiService, SwaggerUIConfig};

// Assuming you have your OpenAPI specs for AuthApi and TestApi in variables auth_spec and test_spec.

// Merge OpenAPI specifications
let merged_spec = merge_openapi_specs(auth_spec, test_spec);

// Create an OpenApiService for the merged specification
let api_service = OpenApiService::new_with_spec(merged_spec, "Merged API", version).server(api_doc_url_info);

// Configure Swagger UI for the merged API
let ui = api_service.swagger_ui(SwaggerUIConfig::default().url("/panel/openapi.json"));

let app = Route::new()
    .at("/status", get(server_status))
    .nest("/api/auth", get_auth_api())
    .nest("/api/test", get_test_api())
    .nest("/panel", ui);

4. Testing:
    
    Make sure to thoroughly test the merged API specification to ensure that it works as expected. Verify that the paths, components, and other relevant information are correctly combined.