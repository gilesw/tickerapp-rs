use tickerapp::{Client, DisclosureQuery, PageQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::io::read_to_string(std::io::stdin())?;
    let page = Client::new(key.trim())
        .disclosures(&DisclosureQuery {
            page: PageQuery {
                page_size: Some(1),
                ..Default::default()
            },
            ..Default::default()
        })
        .await?;
    println!("{}", serde_json::to_string_pretty(&page)?);
    Ok(())
}
