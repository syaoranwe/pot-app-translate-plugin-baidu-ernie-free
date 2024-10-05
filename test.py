import requests
import json

def get_access_token(api_key, secret_key):
    """
    获取access_token
    """
    url = f"https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id={api_key}&client_secret={secret_key}"
    
    payload = json.dumps("")
    headers = {
        'Content-Type': 'application/json',
        'Accept': 'application/json'
    }
    
    response = requests.request("POST", url, headers=headers, data=payload)
    return response.json().get("access_token")

"""info.json中的needs定义
"needs": [{ "key": "api_key", "display": "API Key", "type": "input"},  # 用户必填项
        { "key": "secret_key", "display": "Secret Key", "type": "input"}, # 用户必填项
        { "key": "model_string", "display": "选用模型", "type": "select", "options": {"ernie-lite-8k":"ernie-lite-8k(推荐)", "ernie_speed":"ernie_speed", "ernie-speed-128k":"ernie-speed-128k"}}, # 用户必选项
        { "key": "system_prompt", "display": "System人设", "type": "input"},  # 系统提示词字符串, 留空时默认为You are a professional translation engine.
        { "key": "prompts", "display": "翻译提示词", "type": "input"},  # 用户自定义提示词列表, 由一行json字符串表示, 如果留空则使用默认提示词：[{"role":"user","content":"You are a professional translation engine, skilled in translating text into accurate, professional, fluent, and natural translations, avoiding mechanical literal translations like machine translation. You only translate the text without interpreting it. You only respond with the translated text and do not include any additional content."},{"role":"assistant","content":"OK, I will only translate the text content you provided, never interpret it."},{"role":"user","content":"Translate the text delimited by ``` below to Simplified Chinese(简体中文), only return translation:\n```\nHello, world!\n```\n"},{"role":"assistant","content":"你好，世界！"},{"role":"user","content":"Translate the text delimited by ``` below to English, only return translation:\n```\n再见，小明\n```\n"},{"role":"assistant","content":"Bye, Xiaoming."},{"role":"user","content":"Translate the text delimited by ``` below to $to$, only return translation:\n```\n$src_text$\n```\n"}]
        { "key": "temperature", "display": "temperature", "type": "input"},  # 留空时默认0.6, 范围 (0, 1.0], 不能为0
        { "key": "top_p", "display": "top_p", "type": "input"}, # 留空时默认为0.9, 取值范围：[0.0, 1.0]
        { "key": "penalty_score", "display": "penalty_score", "type": "input"}, # 留空时默认为1.0, 取值范围：[1.0, 2.0]
        { "key": "request_url", "display": "自定义请求地址", "type": "input"}  # 留空时默认为"https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/"
        ],
"""

default_system_prompt = "You are a professional translation engine."
default_prompts = r'''[{"role":"user","content":"You are a professional translation engine, skilled in translating text into accurate, professional, fluent, and natural translations, avoiding mechanical literal translations like machine translation. You only translate the text without interpreting it. You only respond with the translated text and do not include any additional content."},{"role":"assistant","content":"OK, I will only translate the text content you provided, never interpret it."},{"role":"user","content":"Translate the text delimited by ``` below to Simplified Chinese(简体中文), only return translation:\n```\nHello, world!\n```\n"},{"role":"assistant","content":"你好，世界！"},{"role":"user","content":"Translate the text delimited by ``` below to English, only return translation:\n```\n再见，小明\n```\n"},{"role":"assistant","content":"Bye, Xiaoming."},{"role":"user","content":"Translate the text delimited by ``` below to $to$, only return translation:\n```\n$src_text$\n```\n"}]'''
default_temperature = "0.6"
default_top_p = "0.9"
default_penalty_score = "1.0"
default_request_url = "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/"

def translate(text, #待翻译文本字符串
              to,   #目标语言字符串，例如"English"
              needs, #插件需要的其他参数**字典**, 由info.json中的needs定义
              ):

    # 检查needs是否包含必要的参数，如果没有则报错
    try:
        api_key = needs["api_key"]
        secret_key = needs["secret_key"]
        model_string = needs["model_string"]
    except KeyError as e:
        raise KeyError(f"缺少必要参数: {e}")
    
    # 使用needs中的可选参数为变量赋值，如果没有则使用默认值
    system_prompt = needs.get("system_prompt", default_system_prompt)
    prompts = needs.get("prompts", default_prompts)
    temperature = needs.get("temperature", default_temperature)
    top_p = needs.get("top_p", default_top_p)
    penalty_score = needs.get("penalty_score", default_penalty_score)
    request_url = needs.get("request_url", default_request_url)
    
    # 将temperature、top_p、penalty_score转换为浮点数，同时检查是否在范围里，如果不在范围则报错
    try:
        temperature = float(temperature)
        top_p = float(top_p)
        penalty_score = float(penalty_score)
    except ValueError as e:
        raise ValueError(f"参数值转换错误: {e}")
    if not 0 < temperature <= 1.0:
        raise ValueError("temperature参数范围有误，正确的范围是(0, 1.0]")
    if not 0 <= top_p <= 1.0:
        raise ValueError("top_p参数范围有误，正确的范围是[0.0, 1.0]")
    if not 1.0 <= penalty_score <= 2.0:
        raise ValueError("penalty_score参数范围有误，正确的范围是[1.0, 2.0]")
    
    # 构造请求的payload: 将prompts中的$to$替换为to, $src_text$替换为text, 然后转换为json格式payload
    # 将prompts转换为Python的列表
    prompts_list = json.loads(prompts)

    # 在列表中替换$to$和$src_text$
    for prompt in prompts_list:
        if "$to$" in prompt["content"]:
            prompt["content"] = prompt["content"].replace("$to$", to)
        if "$src_text$" in prompt["content"]:
            prompt["content"] = prompt["content"].replace("$src_text$", text)

    # 将列表转换回JSON字符串
    payload_json_data = {
                            "messages": prompts_list,
                            "stream": False,
                            "temperature": temperature,
                            "top_p": top_p,
                            "penalty_score": penalty_score,
                            "system": system_prompt,
                            "max_output_tokens": 2048
                        }
    payload = json.dumps(payload_json_data)
    
    url = f"{request_url}{model_string}?access_token=" + get_access_token(api_key, secret_key)  # 构造请求的url
    headers = {
        'Content-Type': 'application/json'
    }
    
    """bash调用示例
    curl -XPOST 'https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/ernie-lite-8k?access_token=[步骤一调用接口获取的access_token]' -d '{
    "messages": [
        {"role":"user","content":"你好"},
        {"role":"assistant","content":"你好，有什么我可以帮助你的吗？"},
        {"role":"user","content": "我在上海，周末可以去哪里玩？"},
        {"role":"assistant","content": "可以去上海科技馆。"},
        {"role":"user","content": "上海有哪些美食"}
    ]
    }'  | iconv -f utf-8 -t utf-8
    """
    response = requests.request("POST", url, headers=headers, data=payload)
    """
    返回成功示例：
    {
        "result": "机器人回复的译文",
    }
    返回失败示例：
    {
        "error_code": 110,
        "error_msg": "Access token invalid or no longer valid"
    }
    """
    # 返回结果，如果请求失败则返回错误信息
    if response.status_code == 200:
        return response.json().get("result")
    else:
        return response.json()

if __name__ == '__main__':
    # 示例needs
    example_needs = {
        "api_key": "your_api_key",
        "secret_key": "your_secret_key",
        "model_string": "ernie-lite-8k",
        "temperature": "0.1",  # 注意：needs里传入的值都是字符串
    }
    # 调用示例
    print(translate("Expected output: Access token is invalid or has expired.", "Chinese", example_needs))  # 期望输出：访问令牌无效或已过期
