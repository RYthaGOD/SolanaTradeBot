/// Refactored API v2 - Uses boxed filters to overcome type complexity limits
/// Integrates AI Orchestrator for intelligent function routing
use warp::{Filter, Reply};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use crate::ai_orchestrator::{AIOrchestrator, OrchestratorRequest};

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub message: String,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T, message: &str) -> Self {
        Self {
            success: true,
            data,
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OrchestrateRequest {
    context: String,
    parameters: Option<HashMap<String, String>>,
}

/// Create the refactored API with boxed filters
pub fn create_routes(
    orchestrator: Arc<AIOrchestrator>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization"]);

    // Health check endpoint
    let health = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&ApiResponse::new(
                "OK",
                "System is healthy"
            ))
        })
        .boxed();

    // AI Orchestrator endpoint - main entry point for intelligent function routing
    let orchestrate = {
        let orchestrator = orchestrator.clone();
        warp::path("orchestrate")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: OrchestrateRequest| {
                let orchestrator = orchestrator.clone();
                async move {
                    // Get available functions
                    let available_functions = orchestrator.get_available_functions();
                    
                    // Create orchestrator request
                    let orchestrator_req = OrchestratorRequest {
                        context: req.context.clone(),
                        available_functions,
                        current_state: req.parameters.clone().unwrap_or_default(),
                    };
                    
                    // Let AI decide which function to call
                    match orchestrator.decide_action(orchestrator_req).await {
                        Ok(decision) => {
                            // Execute the decided function
                            let params = req.parameters.unwrap_or_default();
                            match orchestrator.execute_function(&decision.recommended_function, params).await {
                                Ok(result) => {
                                    let mut response = HashMap::new();
                                    response.insert("function", decision.recommended_function);
                                    response.insert("result", result);
                                    response.insert("reasoning", decision.reasoning);
                                    response.insert("priority", format!("{:.2}", decision.priority));
                                    
                                    Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                        response,
                                        "Function executed successfully via AI orchestration"
                                    )))
                                }
                                Err(e) => {
                                    Ok(warp::reply::json(&ApiResponse::new(
                                        HashMap::<String, String>::new(),
                                        &format!("Execution failed: {}", e)
                                    )))
                                }
                            }
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Decision failed: {}", e)
                            )))
                        }
                    }
                }
            })
            .boxed()
    };

    // Direct function execution endpoints (bypassing AI decision)
    let execute_function = {
        let orchestrator = orchestrator.clone();
        warp::path!("execute" / String)
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |function_name: String, params: HashMap<String, String>| {
                let orchestrator = orchestrator.clone();
                async move {
                    match orchestrator.execute_function(&function_name, params).await {
                        Ok(result) => {
                            let mut response = HashMap::new();
                            response.insert("function", function_name);
                            response.insert("result", result);
                            
                            Ok::<_, warp::Rejection>(warp::reply::json(&ApiResponse::new(
                                response,
                                "Function executed successfully"
                            )))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&ApiResponse::new(
                                HashMap::<String, String>::new(),
                                &format!("Execution failed: {}", e)
                            )))
                        }
                    }
                }
            })
            .boxed()
    };

    // List available functions
    let list_functions = {
        let orchestrator = orchestrator.clone();
        warp::path("functions")
            .and(warp::get())
            .map(move || {
                let functions = orchestrator.get_available_functions();
                warp::reply::json(&ApiResponse::new(
                    functions,
                    "Available functions"
                ))
            })
            .boxed()
    };

    // Combine all routes using boxed filters
    health
        .or(orchestrate)
        .or(execute_function)
        .or(list_functions)
        .with(cors)
        .with(warp::log("api_v2"))
}

/// Start the API v2 server
pub async fn start_server(orchestrator: Arc<AIOrchestrator>) {
    log::info!("🌐 Starting API v2 Server with AI Orchestration on :8081");
    
    let routes = create_routes(orchestrator);
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8081))
        .await;
}
