use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Get the language following system. Currently we only support Chinese and English,
/// so this will return Language::Chs if system language is Chinese, and will return
/// Language::Eng if system language is not Chinese.
pub fn system_language() -> Language {
    let sys_locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
    if sys_locale.starts_with("zh") {
        Language::Chs
    } else {
        Language::Eng
    }
}

/// Get localized text to display.
pub struct Texts {
    pub language: Language,
    texts: HashMap<Language, HashMap<&'static str, &'static str>>,
}

impl Texts {
    /// Create a new Texts with language settings.
    pub fn new(language: Option<Language>) -> Self {
        let language = language.unwrap_or_else(|| system_language());
        Self { language, texts: HashMap::new() }
            .add("Open Project", "Open Project", "打开项目")
            .add("Browse", "Browse", "浏览")
            .add("Open", "Open", "打开")
            .add("Compile error!", "Compile error!", "编译错误！")
            .add("Compile error message",
                "We have some compile issues, \
                please solve them according to the terminal output, \
                then click 'Project -> Compile' to try again.",
                "我们有一些编译错误，\
                请根据控制台输出解决它们，\
                然后点击“项目->编译”再次编译。")
            .add("Project", "Project", "项目")
            .add("Open", "Open", "打开")
            .add("Close", "Close", "关闭")
            .add("Compile", "Compile", "编译")
            .add("Export", "Export", "导出")
            .add("Scene", "Scene", "场景")
            .add("Save", "Save", "保存")
            .add("Save As", "Save As", "另存为")
            .add("Load", "Load", "加载")
            .add("New", "New", "新建")
            .add("Run", "Run", "运行")
            .add("Start", "Start", "开始")
            .add("Stop", "Stop", "停止")
            .add("Switch to Game Window on Start", "Switch to Game Window on Start", "开始时切换到游戏窗口")
            .add("Ui", "Ui", "界面")
            .add("Current Scale: ", "Current Scale: ", "当前缩放：")
            .add("Enable Dock", "Enable Dock", "启用Dock")
            .add("Disable Dock", "Disable Dock", "禁用Dock")
            .add("fps: ", "fps: ", "帧率：")
            .add("Entities", "Entities", "实体")
            .add("Components", "Components", "组件")
            .add("Uniques", "Uniques", "单例")
            .add("Entity", "Entity", "实体信息")
            .add("Unique", "Unique", "单例信息")
            .add("Language", "Language", "语言")
            .add("en-US", "en-US", "英语")
            .add("zh-CN", "zh-CN", "中文")
            .add("Follow System", "Follow System", "跟随系统")
            .add("Game", "Game", "游戏")
            .add("Edit", "Edit", "编辑")
            .add("Delete", "Delete", "删除")
    }

    fn add(mut self, key: &'static str, eng: &'static str, chs: &'static str) -> Self {
        self.texts.entry(Language::Eng).or_default().insert(key, eng);
        self.texts.entry(Language::Chs).or_default().insert(key, chs);
        self
    }

    /// Get localized text according to key and current language.
    /// If current language is not supported, return "Language not supported".
    /// If key is not found, return "Text key not found".
    pub fn get(&self, key: &str) -> &'static str {
        if let Some(texts) = self.texts.get(&self.language) {
            if let Some(text) = texts.get(key) {
                text
            } else {
                log::error!("Text key not found: {key}, language={:?}", self.language);
                "Text key not found"
            }
        } else {
            log::error!("Language not supported: {:?}", self.language);
            "Language not supported"
        }
    }
}

/// Languages that supported by steel-editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    /// English.
    Eng,
    /// Simplified Chinese.
    Chs,
}
