use crate::server::graphql::{GraphQLContext, Schema};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use juniper::http::{GraphQLBatchRequest, GraphQLRequest};
use std::sync::Arc;

/// GraphQL POST endpoint
pub async fn graphql_handler(
    State(schema): State<Arc<Schema>>,
    Extension(context): Extension<GraphQLContext>,
    Json(request): Json<GraphQLRequest>,
) -> Response {
    let response = request.execute(&schema, &context).await;
    let status = if response.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    (status, Json(response)).into_response()
}

/// GraphQL batch POST endpoint
pub async fn graphql_batch_handler(
    State(schema): State<Arc<Schema>>,
    Extension(context): Extension<GraphQLContext>,
    Json(batch): Json<GraphQLBatchRequest>,
) -> Response {
    let response = batch.execute(&schema, &context).await;
    let status = if response.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    (status, Json(response)).into_response()
}

/// GraphQL playground (GraphiQL)
pub async fn graphql_playground() -> Html<String> {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>GraphQL Playground</title>
    <style>
        body {
            height: 100%;
            margin: 0;
            width: 100%;
            overflow: hidden;
        }
        #graphiql {
            height: 100vh;
        }
    </style>
    <script
        crossorigin
        src="https://unpkg.com/react@18/umd/react.production.min.js"
    ></script>
    <script
        crossorigin
        src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"
    ></script>
    <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css" />
</head>
<body>
    <div id="graphiql">Loading...</div>
    <script
        src="https://unpkg.com/graphiql/graphiql.min.js"
        type="application/javascript"
    ></script>
    <script>
        const fetcher = GraphiQL.createFetcher({
            url: '/graphql',
        });

        ReactDOM.render(
            React.createElement(GraphiQL, { fetcher: fetcher }),
            document.getElementById('graphiql'),
        );
    </script>
</body>
</html>
"#
        .to_string(),
    )
}
