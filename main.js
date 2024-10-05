// 默认的系统提示词
const DEFAULT_SYSTEM_PROMPT = "You are a professional translation engine.";
// 默认的提示词列表，使用JSON格式表示
const DEFAULT_PROMPTS = JSON.stringify([
    {
        role: "user",
        content:
            "You are a professional translation engine, please translate the text into a colloquial, professional, elegant and fluent content, without the style of machine translation. You must only translate the text content, never interpret it.",
    },
    {
        role: "assistant",
        content: "Ok, I will only translate the text content, never interpret it.",
    },
    { role: "user", content: `Translate into Chinese\n"""\nhello\n"""` },
    { role: "assistant", content: "你好" },
    { role: "user", content: `Translate into $to$\n"""\n$src_text$\n"""` }
]);
// 默认的temperature值
const DEFAULT_TEMPERATURE = "0.6";
// 默认的top_p值
const DEFAULT_TOP_P = "0.9";
// 默认的penalty_score值
const DEFAULT_PENALTY_SCORE = "1.0";
// 默认的请求URL
const DEFAULT_REQUEST_URL = "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/";
const DEFAULT_STREAM_SETTING = "true";



// 创建或连接数据库
async function getDatabase(Database) {
    const db = await Database.load('sqlite:history.db');
    // 如果表不存在，则创建表
    await db.execute(
        `CREATE TABLE IF NOT EXISTS baidu_ernie_access_token (
            access_token TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        )`
    );
    return db;
}

// 从数据库读取访问令牌
async function readAccessTokenFromDB(Database) {
    const db = await getDatabase(Database);
    const result = await db.select('SELECT * FROM baidu_ernie_access_token LIMIT 1');
    db.close();

    if (result.length > 0) {
        return {
            access_token: result[0].access_token,
            timestamp: result[0].timestamp
        };
    } else {
        throw new Error("No access token found in the database");
    }
}

// 将访问令牌写入数据库
async function writeAccessTokenToDB(Database, accessToken, timestamp) {
    const db = await getDatabase(Database);
    // 清除旧数据，确保表中只有一条记录
    await db.execute('DELETE FROM baidu_ernie_access_token');
    // 插入新的访问令牌和时间戳
    await db.execute(
        'INSERT INTO baidu_ernie_access_token (access_token, timestamp) VALUES ($1, $2)',
        [accessToken, timestamp]
    );
    db.close();
}

// 调用接口获取最新的访问令牌
async function getNewAccessToken(api_key, secret_key, utils) {
    const url = `https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id=${api_key}&client_secret=${secret_key}`;
    const result = (await utils.http.fetch(url, { method: "POST" })).data;
    if (result.access_token) {
        return result.access_token;
    } else {
        throw new Error("Access token not found in response");
    }
}

// 读取数据库中的访问令牌或者调用接口获取最新的访问令牌
/** @type {(api_key: string, secret_key: string, utils: {http, readBinaryFile, readTextFile, Database, CryptoJS, run, cacheDir, pluginDir, osType}) => Promise<string>} */
async function getAccessToken(api_key, secret_key, utils) {
    const Database = utils.Database;

    try {
        // 从数据库读取访问令牌
        const { access_token, timestamp } = await readAccessTokenFromDB(Database);

        // 获取当前系统时间戳
        const currentTimestamp = Math.floor(Date.now() / 1000);

        // 检查数据库中的时间戳是否过期（过期条件为：当前系统时间戳 - timestamp > 604800，即 7 天）
        if (currentTimestamp - timestamp > 604800) {
            // 如果过期，调用接口获取最新的访问令牌
            const newAccessToken = await getNewAccessToken(api_key, secret_key, utils);
            const newTimestamp = currentTimestamp;

            // 更新数据库中的访问令牌
            await writeAccessTokenToDB(Database, newAccessToken, newTimestamp);
            return newAccessToken;
        } else {
            return access_token;
        }
    } catch (error) {
        // 如果数据库中没有数据，说明是第一次调用，需要调用接口获取最新的访问令牌
        const newAccessToken = await getNewAccessToken(api_key, secret_key, utils);
        const currentTimestamp = Math.floor(Date.now() / 1000);

        // 将新获取的访问令牌写入数据库
        await writeAccessTokenToDB(Database, newAccessToken, currentTimestamp);
        return newAccessToken;
    }
}



// 定义异步翻译函数
async function translate(text, from, to, options) {
    // 从options中解构出config和utils
    // config: config map
    // detect: detected source language
    // setResult: function to set result text
    const { config, detect, setResult, utils } = options;
    // 从utils中解构出功能辅助函数
    // utils: some tools
    //     http: tauri http module
    const { http } = utils;
    // 导入tauri提供的网络请求模块
    const { fetch, Body } = http;

    // 导入插件配置
    // 检查config是否包含必要的参数，如果没有则报错
    const api_key = config.api_key;
    const secret_key = config.secret_key;
    const model_string = config.model_string;

    if (!api_key || !model_string || !secret_key) {
        throw new Error("缺少必要参数: api_key 或 secret_key 或 模型名");
    }
    // 使用config中的可选参数为变量赋值，如果没有则使用默认值
    let request_url = config.request_url || DEFAULT_REQUEST_URL;
    const system_prompt = config.system_prompt || DEFAULT_SYSTEM_PROMPT;
    const prompts = config.prompts || DEFAULT_PROMPTS;
    const temperature = parseFloat(config.temperature || DEFAULT_TEMPERATURE);
    const top_p = parseFloat(config.top_p || DEFAULT_TOP_P);
    const stream = config.stream || DEFAULT_STREAM_SETTING;
    const penalty_score = parseFloat(config.penalty_score || DEFAULT_PENALTY_SCORE);

    // 检查参数值的范围
    if (!(0.0 < temperature && temperature <= 1.0)) {
        throw new Error("temperature参数范围有误, 正确的范围是(0, 1.0]");
    }
    if (!(0.0 <= top_p && top_p <= 1.0)) {
        throw new Error("top_p参数范围有误, 正确的范围是[0, 1.0]");
    }
    if (!(1.0 <= penalty_score && penalty_score <= 2.0)) {
        throw new Error("frequency_penalty参数范围有误, 正确的范围是[1.0, 2.0]");
    }

    // 获取访问令牌
    const access_token = await getAccessToken(api_key, secret_key, utils);
    request_url = `${request_url}${model_string}?access_token=${access_token}`;

    // 如果 request_url 不是以 "http://" 或 "https://" 开头，那么为其添加 "https://" 前缀
    if (!/https?:\/\/.+/.test(request_url)) {
        request_url = `https://${request_url}`;
    }
    const apiUrl = request_url;

    // 将json格式的提示词列表转换为对象数组
    let promptList = JSON.parse(prompts);
    // 最终，map 方法返回一个新的数组，其中每个对象的 content 属性都经过了替换操作。这种方法确保了 promptList 中的每个对象都被正确更新，而不会修改原始数组。
    promptList = promptList.map((item) => {
        return {
            ...item,  // 使用扩展运算符 ...item 创建一个新的对象，保留原始对象的所有属性。
            content: item.content
                .replaceAll('$src_text$', text)
                .replaceAll('$to$', to),  // 注意 to 是 类似 English 这样的字符串，不是语言代码en
        };  // 更新 content 属性，使用 replaceAll 方法替换字符串中的特定占位符。
    });


    // 如果stream是'true'，那么将is_stream设置为true，否则设置为false
    const is_stream = stream === 'true' ? true : false;

    const headers = {
        'Content-Type': 'application/json'
    };

    // 依据不同的模型选择最大输出token数，具体数值参见：https://cloud.baidu.com/doc/WENXINWORKSHOP/s/Jlugqd6pw
    // 当model_string为ernie-3.5-128k或ernie-speed-128k时，max_output_tokens=4096
    // 当model_string为eb-instant、ernie-char-8k时，max_output_tokens=1024
    // 其他模型默认，max_output_tokens=2048
    let max_output_tokens = 2048;
    if (model_string === 'ernie-3.5-128k' || model_string === 'ernie-speed-128k') {
        max_output_tokens = 4096;
    } else if (model_string === 'eb-instant' || model_string === 'ernie-char-8k') {
        max_output_tokens = 1024;
    }
    
    const body = {
        messages: promptList,
        stream: is_stream,
        temperature: temperature,
        top_p: top_p,
        penalty_score: penalty_score,
        system: system_prompt,
        max_output_tokens: max_output_tokens,
    };

    // 如果stream为true，那么调用setResult函数，进行流式输出
    // 注意Tauri 的 http 模块的 fetch 方法并不支持读取响应流，所以需要使用window.fetch
    if (is_stream) {
        const res = await window.fetch(apiUrl, {
            method: 'POST',
            headers: headers,
            body: JSON.stringify(body),
        });

        if (!res.body) {
            throw "ReadableStream not supported by the http client you are using";
        }

        if (res.ok) {
            let target = '';
            const reader = res.body.getReader();
            const decoder = new TextDecoder('utf-8');
            let buffer = '';
            try {
                while (true) {
                    const { done, value } = await reader.read();
                    if (done) break;

                    // 将读取到的二进制数据解码为字符串，并添加到缓冲区
                    buffer += decoder.decode(value, { stream: true });

                    // 查找缓冲区中是否存在完整的事件（以双换行符 '\n\n' 分隔）
                    let boundary = buffer.lastIndexOf('\n\n');
                    // 如果存在完整的事件数据
                    // （判断方法：-1表示没有在buffer中找到'\n\n'。不等于-1代表存在完整的事件数据）
                    if (boundary !== -1) {
                        // 提取完整的事件数据
                        const event = buffer.slice(0, boundary);
                        buffer = buffer.slice(boundary + 2);
                        const chunks = event.split('\n\n');

                        for (const chunk of chunks) {
                            // 去除每个数据块的 'data:' 前缀并去除空白字符
                            const text = chunk.replace(/^data:\s*/, '').trim();
                            if (text === '[DONE]') {
                                continue;
                            }

                            if (text !== '') {
                                try {
                                    const data = JSON.parse(text);
                                    if (data.result) {
                                        target += data.result;
                                        if (setResult) {
                                            setResult(target + '_');
                                        }
                                    }
                                } catch (e) {
                                    console.error('Failed to parse JSON:', e);
                                }
                            }
                        }
                    }
                }
                // 设置最终的翻译结果
                setResult(target.trim());
                return target.trim();
            } finally {
                reader.releaseLock();
            }
        } else {
            throw `Http Request Error\nHttp Status: ${res.status}\n${JSON.stringify(res.data)}`;
        }
    } else {  // 如果stream为false，那么使用fetch的json模式，直接返回所有的翻译结果
        let res = await fetch(apiUrl, {
            method: 'POST',
            headers: headers,
            body: Body.json(body),
        });
        if (res.ok) {
            const result = res.data;
            let resultText = result.result;
            if (resultText) {
                if (resultText.startsWith('"')) {
                    resultText = resultText.slice(1);
                }
                if (resultText.endsWith('"')) {
                    resultText = resultText.slice(0, -1);
                }
                return resultText.trim();
            } else {
                throw new Error("No result in responce: " + JSON.stringify(result));
            }
        } else {
            throw `Http Request Error\nHttp Status: ${res.status}\n${JSON.stringify(res.data)}`;
        }
    }
}