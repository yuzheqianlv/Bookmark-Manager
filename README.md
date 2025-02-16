


# Bookmark Manager

一个使用 Rust 编写的智能书签管理工具，可以自动为书签生成标题和标签，优化浏览器搜索体验。

## 功能特性

- 自动处理 HTML 格式的书签文件
- 使用 Google Gemini AI 分析网页内容
- 智能生成准确的中文标题和标签
- 支持批量处理和断点续传
- 自动保存处理进度
- 优化的浏览器搜索支持
- 完善的错误处理和重试机制

## 环境要求

- Rust 1.75.0 或更高版本
- Google Gemini API 密钥
- 支持 async/await 的 Rust 工具链

## 依赖项

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1.0"
chrono = "0.4"
dotenv = "0.15"
regex = "1.11"
url = "2.5"
scraper = "0.17"
```

## 安装步骤

1. 克隆项目
```bash
git clone https://github.com/yourusername/bookmark-manager.git
cd bookmark-manager
```

2. 配置环境变量
- 创建 `.env` 文件
- 添加 Google API 密钥：
```
GOOGLE_API_KEY=your_api_key_here
```

3. 编译项目
```bash
cargo build --release
```

## 使用方法

1. 准备书签文件
- 从浏览器导出书签为 HTML 格式
- 将文件重命名为 `bookmarks.html`
- 放置在项目根目录

2. 运行程序
```bash
cargo run --release
```

3. 处理结果
- 生成的新书签文件：`bookmarks_with_tags.html`
- 进度保存文件：`bookmarks_progress.html`

## 功能说明

### 批量处理
- 每批处理 5 个书签
- 批次间隔 15 秒
- 自动处理 API 限流

### 标题生成
- 智能分析网页内容
- 生成 15-30 字的中文标题
- 优化搜索关键词

### 标签生成
- 每个书签 3-5 个标签
- 专业领域标签优先
- 过滤通用词汇

### 进度保存
- 每 10 个书签自动保存
- 支持断点续传
- 保存完整元数据

## 错误处理

- API 限流自动等待
- 网络错误自动重试
- 无效 URL 跳过处理
- 详细的错误日志

## 注意事项

1. API 使用限制
- 遵守 Google API 使用配额
- 建议批量处理时间间隔适当调整
- 处理大量书签时注意配额消耗

2. 文件处理
- 备份原始书签文件
- 确保足够磁盘空间
- 定期检查进度文件

3. 网络要求
- 稳定的网络连接
- 合适的代理设置（如需）
- 足够的超时时间

## 常见问题

1. API 密钥问题
```
错误：未找到GOOGLE_API_KEY环境变量
解决：检查 .env 文件配置
```

2. 处理中断
```
进度文件已保存，可直接重新运行程序继续处理
```

3. 标签质量
```
可通过修改 TAG_BLACKLIST 配置过滤无关标签
```

## 开发计划

- [ ] 支持更多书签格式
- [ ] 添加 Web 界面
- [ ] 优化处理速度
- [ ] 支持自定义标签规则
- [ ] 添加导出格式选项

## 贡献指南

欢迎提交 Issue 和 Pull Request，请确保：
1. 代码风格符合 Rust 规范
2. 添加必要的测试
3. 更新相关文档

## 许可证

MIT License

## 作者

[你的名字]

## 更新日志

### v0.1.0 (2024-02-16)
- 初始版本发布
- 基本功能实现
- 支持书签处理

