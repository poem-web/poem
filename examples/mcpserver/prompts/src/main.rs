use poem_mcpserver::{
    Prompts, Tools,
    content::Text,
    prompts::PromptMessages,
    stdio::stdio,
    McpServer,
};

/// A collection of development assistant tools.
struct DevTools {
    /// History of reviewed code snippets
    review_count: u32,
}

/// This server provides development assistant tools for code analysis.
#[Tools]
impl DevTools {
    /// Analyze code complexity and return metrics.
    async fn analyze_complexity(
        &mut self,
        /// The code to analyze
        code: String,
    ) -> Text<String> {
        let lines = code.lines().count();
        let chars = code.len();
        self.review_count += 1;
        Text(format!(
            "Code Analysis #{}\n- Lines: {}\n- Characters: {}\n- Estimated complexity: {}",
            self.review_count,
            lines,
            chars,
            if lines > 50 { "High" } else if lines > 20 { "Medium" } else { "Low" }
        ))
    }

    /// Count occurrences of a pattern in code.
    async fn count_pattern(
        &self,
        /// The code to search in
        code: String,
        /// The pattern to search for
        pattern: String,
    ) -> Text<String> {
        let count = code.matches(&pattern).count();
        Text(format!("Found {} occurrences of '{}'", count, pattern))
    }

    /// Get the total number of code reviews performed.
    async fn get_review_count(&self) -> Text<u32> {
        Text(self.review_count)
    }
}

/// A collection of development assistant prompts.
struct DevPrompts {
    /// The assistant's persona name
    assistant_name: String,
}

/// This server provides development assistant prompts for code review,
/// documentation generation, and debugging help.
///
/// Use the 'code_review' prompt for reviewing code snippets.
/// Use the 'generate_docs' prompt for generating documentation.
/// Use the 'debug_help' prompt for debugging assistance.
#[Prompts]
impl DevPrompts {
    /// Review code for potential issues, style, and best practices.
    async fn code_review(
        &self,
        /// The code snippet to review
        #[mcp(required)]
        code: Option<String>,
        /// The programming language of the code
        language: Option<String>,
        /// Focus area: "security", "performance", "style", or "all"
        focus: Option<String>,
    ) -> PromptMessages {
        let lang = language.unwrap_or_else(|| "unknown".to_string());
        let focus_area = focus.unwrap_or_else(|| "all".to_string());
        
        PromptMessages::new()
            .user(Text(format!(
                "Please review the following {} code. Focus on: {}\n\n```{}\n{}\n```",
                lang, focus_area, lang, code.unwrap()
            )))
            .assistant(Text(format!(
                "I'm {}, and I'll review this {} code focusing on {}. Let me analyze it...",
                self.assistant_name, lang, focus_area
            )))
    }

    /// Generate documentation for a code snippet.
    async fn generate_docs(
        &self,
        /// The code to document
        #[mcp(required)]
        code: Option<String>,
        /// Documentation style: "markdown", "jsdoc", "rustdoc", etc.
        style: Option<String>,
    ) -> PromptMessages {
        let doc_style = style.unwrap_or_else(|| "markdown".to_string());
        
        PromptMessages::new()
            .user(Text(format!(
                "Generate {} documentation for the following code:\n\n```\n{}\n```",
                doc_style, code.unwrap()
            )))
    }

    /// Get help debugging an issue.
    async fn debug_help(
        &self,
        /// Description of the problem
        #[mcp(required)]
        problem: Option<String>,
        /// The error message, if any
        error_message: Option<String>,
        /// Relevant code snippet
        code: Option<String>,
    ) -> PromptMessages {
        let mut prompt = format!("I need help debugging an issue.\n\nProblem: {}", problem.unwrap());
        
        if let Some(err) = error_message {
            prompt.push_str(&format!("\n\nError message:\n```\n{}\n```", err));
        }
        
        if let Some(code_snippet) = code {
            prompt.push_str(&format!("\n\nRelevant code:\n```\n{}\n```", code_snippet));
        }
        
        PromptMessages::new()
            .user(Text(prompt))
            .assistant(Text(format!(
                "I'm {} and I'll help you debug this issue. Let me analyze the problem...",
                self.assistant_name
            )))
    }

    /// Get a simple greeting from the assistant.
    async fn greet(&self) -> String {
        format!(
            "Hello! I'm {}, your development assistant. I can help you with:\n\
            - Code reviews (use 'code_review' prompt)\n\
            - Documentation generation (use 'generate_docs' prompt)\n\
            - Debugging help (use 'debug_help' prompt)\n\n\
            How can I assist you today?",
            self.assistant_name
        )
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let tools = DevTools { review_count: 0 };
    let prompts = DevPrompts {
        assistant_name: "CodeBot".to_string(),
    };
    
    stdio(McpServer::new().tools(tools).prompts(prompts)).await
}
