use poem_mcpserver::{
    McpServer, Tools,
    content::Text,
    stdio::stdio,
    tool::StructuredContent,
};
use schemars::JsonSchema;
use serde::Serialize;

struct ColorTools {
    color: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ColorState {
    color: String,
}

/// Minimal MCP Apps demo with a UI resource.
#[Tools]
impl ColorTools {
    /// Open the color picker UI.
    #[mcp(ui_resource = "ui://apps/color-picker")]
    async fn color_app(&self) -> StructuredContent<ColorState> {
        StructuredContent(ColorState {
            color: self.color.clone(),
        })
    }

    /// Get the current color.
    async fn get_color(&self) -> StructuredContent<ColorState> {
        StructuredContent(ColorState {
            color: self.color.clone(),
        })
    }

    /// Update the current color.
    async fn set_color(&mut self, color: String) -> Text<String> {
        self.color = color.clone();
        Text(self.color.clone())
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let server = McpServer::new()
        .ui_resource(
            "ui://apps/color-picker",
            "Color Picker",
            "Simple color picker UI powered by @modelcontextprotocol/ext-apps.",
            "text/html",
            include_str!("../ui/index.html"),
        )
        .tools(ColorTools {
            color: "#ff0000".to_string(),
        });

    stdio(server).await
}
