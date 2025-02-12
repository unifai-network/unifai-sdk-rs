use std::env;
use unifai_sdk::{
    rig::{
        completion::{Completion, Message},
        message::{AssistantContent, Text, ToolResult, ToolResultContent, UserContent},
        providers::openai,
        OneOrMany,
    },
    tools::get_tools,
};

#[tokio::test]
async fn test_tools_with_openai() {
    tracing_subscriber::fmt().init();

    let unifai_agent_api_key =
        env::var("UNIFAI_AGENT_API_KEY").expect("UNIFAI_AGENT_API_KEY not set");
    let (search_tools, call_tool) = get_tools(unifai_agent_api_key);

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let openai_client = openai::Client::new(&openai_api_key);
    let agent = openai_client
        .agent(openai::GPT_4O)
        .preamble(concat!(
            "You are a personal assistant capable of doing many things with your tools. ",
            "When you are given a task you cannot do (like something you don't know, ",
            "or requires you to take some action), try find appropriate tools to do it."
        ))
        .tool(search_tools)
        .tool(call_tool)
        .build();

    let prompt = concat!(
        "Get the balance of Solana account 11111111111111111111111111111111. ",
        "If the balance is greater than zero, output 'Unifai!'."
    );
    let mut chat_history = vec![Message::user(prompt)];

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

    assert!(result.text.contains("Unifai!"));
}
