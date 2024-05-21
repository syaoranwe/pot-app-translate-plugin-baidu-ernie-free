use std::collections::HashMap;
use std::error::Error;
use reqwest::blocking::Client;
use reqwest::header;
use serde_json::{json, Value};

// 获取access_token的函数
#[no_mangle]
pub fn get_access_token(api_key: &str, secret_key: &str) -> Result<String, Box<dyn Error>> {
    // 创建一个新的HTTP客户端
    let client = Client::new();

    // 构建请求URL
    let url = format!(
        "https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id={}&client_secret={}",
        api_key, secret_key
    );

    // 设置请求头
    let mut headers = header::HeaderMap::new();
    headers.insert("Content-Type", header::HeaderValue::from_static("application/json"));
    headers.insert("Accept", header::HeaderValue::from_static("application/json"));

    // 发送HTTP POST请求并获取响应
    let res = client.post(&url).headers(headers).send()?;

    // 获取响应状态码
    let status = res.status();
    match status {
        // 如果状态码是200 OK，则解析响应文本
        reqwest::StatusCode::OK => {
            let result_text = res.text()?; // 获取响应文本
            let result: Value = serde_json::from_str(&result_text)?; // 将响应文本解析为JSON
            // 提取access_token并返回
            if let Some(access_token) = result.get("access_token").and_then(Value::as_str) {
                return Ok(access_token.to_string());
            } else {
                return Err("Failed to extract access_token from response".into());
            }
        }
        // 如果状态码不是200 OK，则返回错误信息
        _ => {
            return Err(format!("Error {}: {}", status, res.text()?).into());
        }
    }
}

#[no_mangle]
pub fn translate(
    text: &str, // 待翻译文本
    from: &str, // 源语言
    to: &str,   // 目标语言
    // (pot会根据info.json 中的 language 字段传入插件需要的语言代码，无需再次转换)
    detect: &str, // 检测到的语言 (若使用 detect, 需要手动转换)
    needs: HashMap<String, String>, // 插件需要的其他参数,由info.json定义
) -> Result<String, Box<dyn Error>> {
    // 检查并提取必要的参数
    let api_key = needs.get("api_key").ok_or("缺少必要参数: api_key")?;
    let secret_key = needs.get("secret_key").ok_or("缺少必要参数: secret_key")?;
    let model_string = needs.get("model_string").ok_or("缺少必要参数: model_string")?;

    // 提取可选参数，若不存在则使用默认值
    let system_prompt = needs.get("system_prompt").unwrap_or(&default_system_prompt());
    let prompts = needs.get("prompts").unwrap_or(&default_prompts());
    let temperature = needs.get("temperature").unwrap_or(&default_temperature()).parse::<f64>()?;
    let top_p = needs.get("top_p").unwrap_or(&default_top_p()).parse::<f64>()?;
    let penalty_score = needs.get("penalty_score").unwrap_or(&default_penalty_score()).parse::<f64>()?;
    let request_url = needs.get("request_url").unwrap_or(&default_request_url());

    // 检查参数范围
    if temperature <= 0.0 || temperature > 1.0 {
        return Err("temperature参数范围有误，正确的范围是(0, 1.0]".into());
    }
    if top_p < 0.0 || top_p > 1.0 {
        return Err("top_p参数范围有误，正确的范围是[0.0, 1.0]".into());
    }
    if penalty_score < 1.0 || penalty_score > 2.0 {
        return Err("penalty_score参数范围有误，正确的范围是[1.0, 2.0]".into());
    }

    // 获取access_token
    let access_token = get_access_token(api_key, secret_key)?;

    // 构建请求的payload
    let mut prompts_list: Vec<Value> = serde_json::from_str(prompts)?;
    for prompt in &mut prompts_list {
        if let Some(content) = prompt.get_mut("content").and_then(Value::as_str_mut) {
            *content = content.replace("$to$", to).replace("$src_text$", text);
        }
    }
    
    let payload = json!({
        "messages": prompts_list,
        "stream": false,
        "temperature": temperature,
        "top_p": top_p,
        "penalty_score": penalty_score,
        "system": system_prompt,
        "max_output_tokens": 2048
    });

    // 构建请求URL
    let url = format!("{}{}?access_token={}", request_url, model_string, access_token);

    // 设置请求头
    let mut headers = header::HeaderMap::new();
    headers.insert("Content-Type", header::HeaderValue::from_static("application/json"));

    // 发送HTTP POST请求并获取响应
    let client = Client::new();
    let res = client.post(&url).headers(headers).json(&payload).send()?;

    // 解析响应
    if res.status().is_success() {
        let result: Value = res.json()?;
        if let Some(translated_text) = result.get("result").and_then(Value::as_str) {
            return Ok(translated_text.to_string());
        } else {
            return Err("Failed to extract translated text from response".into());
        }
    } else {
        let error_msg = res.text()?;
        return Err(format!("Error {}: {}", res.status(), error_msg).into());
    }
}

// 默认参数值函数
fn default_system_prompt() -> String {
    "You are a professional translation engine.".to_string()
}

fn default_prompts() -> String {
    r#"[{"role":"user","content":"You are a professional translation engine, skilled in translating text into accurate, professional, fluent, and natural translations, avoiding mechanical literal translations like machine translation. You only translate the text without interpreting it. You only respond with the translated text and do not include any additional content."},{"role":"assistant","content":"OK, I will only translate the text content you provided, never interpret it."},{"role":"user","content":"Translate the text delimited by ``` below to Simplified Chinese(简体中文), only return translation:\n```\nHello, world!\n```\n"},{"role":"assistant","content":"你好，世界！"},{"role":"user","content":"Translate the text delimited by ``` below to English, only return translation:\n```\n再见，小明\n```\n"},{"role":"assistant","content":"Bye, Xiaoming."},{"role":"user","content":"Translate the text delimited by ``` below to $to$, only return translation:\n```\n$src_text$\n```\n"}]"#.to_string()
}

fn default_temperature() -> String {
    "0.6".to_string()
}

fn default_top_p() -> String {
    "0.9".to_string()
}

fn default_penalty_score() -> String {
    "1.0".to_string()
}

fn default_request_url() -> String {
    "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn try_request() {
        let mut needs = HashMap::new();
        needs.insert("requestPath".to_string(), "lingva.pot-app.com".to_string());
        let result = translate("你好，世界！", "auto", "en", "zh_cn", needs).unwrap();
        println!("{result}");
    }
}
