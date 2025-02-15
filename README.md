## uni-llm-api: The Unified LLM API Layer
**One interface, consistent results, powered by Rust.**

### Background  
After using the free `deepseek-r1` API provided by various cloud vendors and integrating it into the frontend, I noticed that the display process was not smooth and occasionally froze. However, when I directly called the stream API using `curl`, this issue did not occur. Upon investigation, I found that the problem was caused by inconsistencies in how different APIs handle and return the chain-of-thought results. This led me to start developing this tool to address the issue.  

Here are a few platforms I'm currently using that offer free API access:  
- [deepseek-platform](https://platform.deepseek.com/usage): Offers a 10 CNY credit  
- [siliconflow](https://docs.siliconflow.cn/api-reference/chat-completions/chat-completions): Offers a 14 CNY credit  
- [alibaba cloud](https://www.aliyun.com/product/bailian): Offers 10 million tokens, expiring after six months  
- [bytedance cloud](https://www.volcengine.com/docs/82379/1099522): Offers 500,000 tokens per model  
- [tencent cloud](https://cloud.tencent.com/document/product/1772/115969): Free access until February 26  
- [google gemini](https://ai.google.dev/gemini-api/docs/text-generation): Totally free
...

### Features  

- **Unified Interface** : Standardizes APIs to match the [Ollama API](https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-completion),and specifically tailored the experience for `OpenWebUI`.
- **High Performance** : After comparing the performance and speed of directly adapting plugins through [`OpenWebUI`](https://docs.openwebui.com/pipelines/)'s API with this tool, we've observed a performance boost of approximately 50%.
- **API Proxying** : This feature allows for custom proxy configurations for each individual API provider, which is incredibly useful for users within China accessing APIs from services like `Gemini` or `OpenAI`.
- **Easy to manage API calls** : By storing API keys and preferred models for various vendors in a single JSON file, and then exposing a unified API for external tools to call, we've significantly streamlined model integration for local AI tools.

### QuickStart

#### Install

#### Usage


#### Integrated into other AI tools

##### OpenWebUi
