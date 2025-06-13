use poem_mcpserver::{content::Text, stdio::stdio, McpServer, Tools};

struct Counter {
    count: i32,
}

/// This server provides a counter tool that can increment and decrement values.
///
/// The counter starts at 0 and can be modified using the 'increment' and
/// 'decrement' tools. Use 'get_value' to check the current count.
#[Tools]
impl Counter {
    /// Increment the counter by 1
    async fn increment(&mut self) -> Text<i32> {
        self.count += 1;
        Text(self.count)
    }

    /// Decrement the counter by 1
    async fn decrement(&mut self) -> Text<i32> {
        self.count -= 1;
        Text(self.count)
    }

    /// Get the current counter value
    async fn get_value(&self) -> Text<i32> {
        Text(self.count)
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    stdio(McpServer::new().tools(Counter { count: 0 })).await
}
