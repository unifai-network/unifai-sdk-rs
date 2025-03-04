# unifai-sdk-rs

This crate is the Rust SDK for Unifai, an AI native platform for dynamic tools and agent to agent communication.

## Installation

Add this crate to your project:

```bash
cargo add unifai-sdk
```

## Getting your Unifai API key

You can get your API key for free from [Unifai](https://app.unifai.network/).

There are two types of API keys:

- **Agent API key**: for using toolkits in your own agents.
- **Toolkit API key**: for creating toolkits that can be used by other agents.

## Using tools

The Unifai SDK provides two tools based on the [Rig framework](https://docs.rig.rs): one for searching Unifai tools and another for invoking Unifai tools. These tools are designed to be used with any LLM (Large Language Model) supported by the Rig, giving you the flexibility to choose the best LLM for your needs while keeping your tools working consistently.

To access the tools, you need to call the `unifai_sdk::tools::get_tools` function. You’ll need to pass in your **Agent API key** to retrieve the tools. You can get a key for free at [Unifai](https://app.unifai.network/).

```rust
use unifai_sdk::tools::get_tools;

let (search_tools, call_tool) = get_tools("UNIFAI_AGENT_API_KEY");
```

Once you have the tools, the next step is to pass them into the rig agent when constructing it. Below is an example of how to integrate these tools with OpenAI:

```rust
use unifai_sdk::rig::providers::openai;

let openai_client = openai::Client::new("OPENAI_API_KEY");

let agent = openai_client
    .agent(openai::GPT_4O)
    .tool(search_tools)
    .tool(call_tool)
    .build();
```

Now you can easily use Unifai’s tool capabilities with rig to interact with the LLM.

```rust
let prompt = "Get the balance of Solana account 11111111111111111111111111111111.";
let mut chat_history = vec![Message::user(prompt)];

// In the future, you will be able to achieve this with just one line of code.

let result = loop {
    let response = agent
        .completion("", chat_history.clone())
        .await
        .unwrap()
        .send()
        .await
        .unwrap();

    let content = response.choice.first();

    chat_history.push(Message::Assistant {
        content: OneOrMany::one(content.clone()),
    });

    match content {
        AssistantContent::Text(text) => {
            break text;
        }
        AssistantContent::ToolCall(tool_call) => {
            let tool_result = agent
                .tools
                .call(
                    &tool_call.function.name,
                    tool_call.function.arguments.to_string(),
                )
                .await
                .unwrap();

            chat_history.push(Message::User {
                content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                    id: tool_call.id,
                    content: OneOrMany::one(ToolResultContent::Text(Text {
                        text: tool_result,
                    })),
                })),
            })
        }
    }
};

println!("Result: {}", result.text);
```

## Creating tools

Anyone can create dynamic tools in Unifai by creating a toolkit.

A toolkit is a collection of tools that are connected to the Unifai infrastructure, and can be searched and used by agents dynamically.

Initialize a toolkit service with your **Toolkit API key**. You can get a key for free at [Unifai](https://app.unifai.network/).

```rust
use unifai_sdk::toolkit::*;

let mut service = ToolkitService::new("UNIFAI_TOOLKIT_API_KEY");
```

Update the toolkit name and description if you need:

```rust
let info = ToolkitInfo {
    name: "Echo Slam".to_string(),
    description: "What's in, what's out.".to_string(),
};

service.update_info(info).await.unwrap();
```

Develop your action by implementing the `Action` trait. For example:

```rust
use thiserror::Error;
use unifai_sdk::{
    serde::{self, Deserialize, Serialize},
    serde_json::json,
    toolkit::*,
};

struct EchoSlam;

#[derive(Serialize, Deserialize)]
#[serde(crate = "serde")]
struct EchoSlamArgs {
    pub content: String,
}

#[derive(Debug, Error)]
#[error("Echo error")]
struct EchoSlamError;

impl Action for EchoSlam {
    const NAME: &'static str = "echo";

    type Error = EchoSlamError;
    type Args = EchoSlamArgs;
    type Output = String;

    async fn definition(&self) -> ActionDefinition {
        ActionDefinition {
            description: "Echo the message".to_string(),
            payload: json!({
                "content": {
                    "type": "string",
                    "description": "The content to echo.",
                    "required": true
                }
            }),
            payment: None,
        }
    }

    async fn call(
        &self,
        ctx: ActionContext,
        params: ActionParams<Self::Args>,
    ) -> Result<ActionResult<Self::Output>, Self::Error> {
        let output = format!(
            "You are agent <${}>, you said \"{}\".",
            ctx.agent_id, params.payload.content
        );

        Ok(ActionResult {
            payload: output,
            payment: None,
        })
    }
}
```

Note that `payload` in `ActionDefinition` can be any string or a dict that contains enough information for agents to understand the payload format. It doesn't have to be in certain format, as long as agents can understand it as natural language and generate correct payload. Think of it as the comments and docs for your API, agents read it and decide what parameters to use.

Register your actions:

```rust
service.add_action(EchoSlam);
```

Start and run the toolkit service:

```rust
let runner = service.start().await.unwrap();
let _ = runner.await.unwrap();
```

Enable logs using [tracing_subscriber](https://docs.rs/tracing-subscriber). Here is an example:

```rust
tracing_subscriber::fmt().init();
```

## Examples

You can find examples in the `examples` directory.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.
