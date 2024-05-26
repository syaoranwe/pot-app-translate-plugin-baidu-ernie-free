use dirs::config_dir;
use reqwest::header;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

// 默认的系统提示词
const DEFAULT_SYSTEM_PROMPT: &str = "You are a professional translation engine.";
// 默认的提示词列表，使用JSON格式表示
const DEFAULT_PROMPTS: &str = r#"[{"role":"user","content":"You are a professional translation engine, skilled in translating text into accurate, professional, fluent, and natural translations, avoiding mechanical literal translations like machine translation. You only translate the text without interpreting it. You only respond with the translated text and do not include any additional content."},{"role":"assistant","content":"OK, I will only translate the text content you provided, never interpret it."},{"role":"user","content":"Translate the text delimited by ``` below to Simplified Chinese(简体中文), only return translation:\n```\nHello, world!\n```\n"},{"role":"assistant","content":"你好，世界！"},{"role":"user","content":"Translate the text delimited by ``` below to English, only return translation:\n```\n再见，小明\n```\n"},{"role":"assistant","content":"Bye, Xiaoming."},{"role":"user","content":"Translate the text delimited by ``` below to $to$, only return translation:\n```\n$src_text$\n```\n"}]"#;
// 默认的temperature值
const DEFAULT_TEMPERATURE: &str = "0.6";
// 默认的top_p值
const DEFAULT_TOP_P: &str = "0.9";
// 默认的penalty_score值
const DEFAULT_PENALTY_SCORE: &str = "1.0";
// 默认的请求URL
const DEFAULT_REQUEST_URL: &str = "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/";

// 调用接口获取最新的访问令牌
fn get_new_access_token(api_key: &str, secret_key: &str) -> Result<String, Box<dyn Error>> {
    let url = format!("https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id={}&client_secret={}", api_key, secret_key);
    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).send()?;
    let result: Value = response.json()?;
    match result.get("access_token") {
        Some(token) => Ok(token.as_str().unwrap().to_string()),
        None => Err("Access token not found in response".into()),
    }
}

// 读取本地缓存的访问令牌或者调用接口获取最新的访问令牌
fn get_access_token(api_key: &str, secret_key: &str) -> Result<String, Box<dyn Error>> {
    // 首先拼接缓存令牌的 JSON 文件路径 config_dir() + com.pot-app.desktop/plugins/translate/[plugin].com.pot-app.baidu-ernie-free/access_token.json
    let config_dir_path = config_dir().unwrap();
    let access_token_cache_path = config_dir_path
        .join("com.pot-app.desktop")
        .join("plugins")
        .join("translate")
        .join("[plugin].com.pot-app.baidu-ernie-free")
        .join("access_token.json");
    // 尝试读取文件
    match read_to_string(&access_token_cache_path) {
        Ok(content) => {
            // 如果文件存在，解析 JSON 内容
            // println!("发现缓存的访问令牌，尝试读取...");  // 调试用
            let json_value: Value = serde_json::from_str(&content)?;
            let access_token = json_value["access_token"].as_str().unwrap().to_string();
            let timestamp = json_value["timestamp"].as_u64().unwrap();
            // 获取当前系统时间戳
            let current_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            // 检查文件中的时间戳是否过期（过期条件为：当前系统时间戳 - timestamp > 604800，即 7 天）
            if current_timestamp - timestamp > 604800 {
                // println!("访问令牌已过期，尝试获取新的访问令牌...");  // 调试用
                // 如果过期，调用接口获取最新的访问令牌
                let new_access_token = get_new_access_token(api_key, secret_key)?;
                // 创建新的 JSON 对象
                let new_json_value = json!({
                    "access_token": new_access_token,
                    "timestamp": current_timestamp
                });
                // 将新的访问令牌和当前系统时间戳写入文件进行更新
                let mut file = File::create(&access_token_cache_path)?;
                file.write_all(new_json_value.to_string().as_bytes())?;
                // 返回新的访问令牌
                Ok(new_access_token)
            } else {
                // println!("访问令牌未过期，直接返回...");  // 调试用
                // 如果未过期，直接返回文件中的访问令牌
                Ok(access_token)
            }
        }
        Err(_) => {
            // println!("未发现缓存的访问令牌，可能是第一次安装使用，尝试获取新的访问令牌...");  // 调试用
            // 如果文件不存在，说明是插件安装后第一次调用，需要调用接口获取最新的访问令牌
            let new_access_token = get_new_access_token(api_key, secret_key)?;
            // 获取当前系统时间戳
            let current_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            // 创建新的 JSON 对象
            let new_json_value = json!({
                "access_token": new_access_token,
                "timestamp": current_timestamp
            });
            // 将访问令牌和当前系统时间戳写入文件
            let mut file = File::create(&access_token_cache_path)?;
            file.write_all(new_json_value.to_string().as_bytes())?;
            // 返回访问令牌
            Ok(new_access_token)
        }
    }
}

#[no_mangle]
pub fn translate(
    text: &str,
    _from: &str,
    to: &str,
    _detect: &str,
    needs: HashMap<String, String>,
) -> Result<Value, Box<dyn Error>> {
    // 检查needs是否包含必要的参数，如果没有则报错
    let api_key = needs.get("api_key").ok_or("缺少必要参数: api_key")?;
    let secret_key = needs.get("secret_key").ok_or("缺少必要参数: secret_key")?;
    let model_string = needs.get("model_string").ok_or("缺少必要参数: model_string")?;

    // 使用needs中的可选参数为变量赋值，如果没有则使用默认值
    // 使用.to_owned() 将字符串字面量转换为String类型
    let system_prompt = needs.get("system_prompt").map(String::to_owned).unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_owned());
    let prompts = needs.get("prompts").map(String::to_owned).unwrap_or_else(|| DEFAULT_PROMPTS.to_owned());
    let temperature = needs.get("temperature").map(String::to_owned).unwrap_or_else(|| DEFAULT_TEMPERATURE.to_owned());
    let top_p = needs.get("top_p").map(String::to_owned).unwrap_or_else(|| DEFAULT_TOP_P.to_owned());
    let penalty_score = needs.get("penalty_score").map(String::to_owned).unwrap_or_else(|| DEFAULT_PENALTY_SCORE.to_owned());
    let request_url = needs.get("request_url").map(String::to_owned).unwrap_or_else(|| DEFAULT_REQUEST_URL.to_owned());

    // 将temperature、top_p、penalty_score转换为浮点数，同时检查是否在范围里，如果不在范围则报错
    let temperature: f64 = temperature.parse().map_err(|_| "temperature参数值转换错误")?;
    let top_p: f64 = top_p.parse().map_err(|_| "top_p参数值转换错误")?;
    let penalty_score: f64 = penalty_score.parse().map_err(|_| "penalty_score参数值转换错误")?;

    if !(0.0 < temperature && temperature <= 1.0) {
        return Err("temperature参数范围有误，正确的范围是(0, 1.0]".into());
    }
    if !(0.0 <= top_p && top_p <= 1.0) {
        return Err("top_p参数范围有误，正确的范围是[0.0, 1.0]".into());
    }
    if !(1.0 <= penalty_score && penalty_score <= 2.0) {
        return Err("penalty_score参数范围有误，正确的范围是[1.0, 2.0]".into());
    }

    // 构造请求的payload: 将prompts中的$to$替换为to, $src_text$替换为text, 然后转换为json格式payload
    // 将prompts转换为Value类型
    let prompts_value: Value = serde_json::from_str(&prompts)?;

    // 在prompts中替换$to$和$src_text$
    let prompts_list = prompts_value.as_array().ok_or("提示词列表格式有误")?;
    let mut new_prompts_list = Vec::new();
    for prompt in prompts_list {
        let mut new_prompt = prompt.clone();
        if let Some(content) = new_prompt.get("content").and_then(|v| v.as_str()) {
            let new_content = content.replace("$to$", to).replace("$src_text$", text);
            new_prompt["content"] = json!(new_content);
        }
        new_prompts_list.push(new_prompt);
    }

    // 构造请求的payload
    let payload = json!({
        "messages": new_prompts_list,
        "stream": false,
        "temperature": temperature,
        "top_p": top_p,
        "penalty_score": penalty_score,
        "system": system_prompt,
        "max_output_tokens": 2048
    });

    // 构造请求的url
    let access_token = get_access_token(api_key, secret_key)?;
    let url = format!("{}{model_string}?access_token={access_token}", request_url);

    // 发送请求并处理响应
    let client = reqwest::blocking::ClientBuilder::new().build()?;
    let response = client
        .post(&url)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()?;

    // 返回结果，如果请求失败则返回错误信息
    if response.status().is_success() {
        let result: Value = response.json()?;
        match result.get("result") {
            Some(result_text) => Ok(result_text.clone()),
            None => Err("响应中未找到翻译结果".into()),
        }
    } else {
        let error_msg = response.text().unwrap_or_else(|_| "请求失败".to_string());
        Err(format!("请求失败: {}", error_msg).into())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn try_request() {
        let mut needs = HashMap::new();
        needs.insert("api_key".to_string(), "your_api_key".to_string());
        needs.insert("secret_key".to_string(), "your_secret_key".to_string());
        needs.insert("model_string".to_string(), "ernie-lite-8k".to_string());
        needs.insert("temperature".to_string(), "0.1".to_string());
        let result = translate("你好，世界！", "auto", "en", "zh_cn", needs).unwrap();
        println!("{result}");
    }
}
