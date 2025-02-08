## uni-llm-api: The Unified LLM API Layer
**One interface, consistent results, powered by Rust.**

### Background  
After using the free `deepseek-r1` API provided by various cloud vendors and integrating it into the frontend, I noticed that the display process was not smooth and occasionally froze. However, when I directly called the stream API using `curl`, this issue did not occur. Upon investigation, I found that the problem was caused by inconsistencies in how different APIs handle and return the chain-of-thought results. This led me to start developing this tool to address the issue.  

Currently, platforms where you can freely access deepseek-r1 include:  
- [deepseek-platform](https://platform.deepseek.com/usage): Offers a 10 CNY credit  
- [siliconflow](https://docs.siliconflow.cn/api-reference/chat-completions/chat-completions): Offers a 14 CNY credit  
- [alibaba cloud](https://www.aliyun.com/product/bailian): Offers 10 million tokens, expiring after six months  
- [bytedance cloud](https://www.volcengine.com/docs/82379/1099522): Offers 500,000 tokens per model  
- [tencent cloud](https://cloud.tencent.com/document/product/1772/115969): Free access until February 26  
...

### Features  

- **Unified Interface** : Standardizes APIs to match the [Ollama API](https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-completion)
- **High Performance** : Built in Rust for fast, reliable, and resource-efficient operations
- **Cross-Platform Support** : Simplifies integration with multiple LLM providers

### Usage

TODO