use serde::Deserialize;
use worker::*;

#[derive(Deserialize)]
struct JsonRpc {
    id: i32,
}

pub enum Chain {
    Sepolia,
    Optimism,
}

impl Chain {
    pub fn api_subdomain(&self) -> &str {
        match self {
            Chain::Sepolia => "eth-sepolia",
            Chain::Optimism => "opt-mainnet",
        }
    }
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> worker::Result<Response> {
    let router = Router::new();
    router
        .post_async("/sepolia", |req, ctx| async move {
            handle_graphql_request(req, ctx, Chain::Sepolia).await
        })
        .post_async("/optimism", |req, ctx| async move {
            handle_graphql_request(req, ctx, Chain::Optimism).await
        })
        .run(req, env)
        .await
}

pub async fn handle_graphql_request(
    mut req: Request,
    ctx: RouteContext<()>,
    chain: Chain,
) -> worker::Result<Response> {
    let body = req.text().await?;
    let json: JsonRpc = serde_json::from_str(&body).unwrap();

    let cache_key = format!(
        "https://{}.g.alchemy.com/v2/{}",
        chain.api_subdomain(),
        json.id
    );
    let cache = Cache::default();
    let maybe_response = cache.get(&cache_key, false).await?;
    if let Some(response) = maybe_response {
        console_log!("Cache hit");
        return Ok(response);
    }

    let api_key = match ctx.env.secret("ALCHEMY_API_KEY") {
        Ok(key) => key,
        Err(_) => return Err("ALCHEMY_API_KEY secret is missing".into()),
    };

    let query_url = format!(
        "https://{}.g.alchemy.com/v2/{}",
        chain.api_subdomain(),
        api_key.to_string()
    );

    let mut headers = Headers::new();
    headers.append("Content-Type", "application/json")?;
    headers.append("User-Agent", "c-atts/0.0.1")?;

    let mut init = RequestInit::new();
    init.with_headers(headers);
    init.with_method(Method::Post);
    init.with_body(Some(body.into()));

    let query_request = Request::new_with_init(&query_url, &init)?;
    let query_response = Fetch::Request(query_request).send().await;

    match query_response {
        Ok(query_response) => {
            let mut response = Response::from_body(query_response.body().to_owned())?;
            response
                .headers_mut()
                .append("Content-Type", "application/json")?;
            let cloned_response = response.cloned()?;
            cache.put(&cache_key, cloned_response).await?;
            return Ok(response);
        }
        Err(e) => return Response::error(e.to_string(), 500),
    }
}
