use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        Self {
            language,
            texts: HashMap::new(),
        }
        .add("Open Project", "Open Project", "打开项目")
        .add("Browse", "Browse", "浏览")
        .add("Open", "Open", "打开")
        .add("Compile error!", "Compile error!", "编译错误！")
        .add(
            "Compile error message",
            "We have some compile issues,\
                please solve them according to the terminal output,\
                then click 'Project -> Compile' to try again.",
            "我们有一些编译错误，\
                请根据控制台输出解决它们，\
                然后点击“项目->编译”再次编译。",
        )
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
        .add(
            "Switch to Game Window on Start",
            "Switch to Game Window on Start",
            "开始时切换到游戏窗口",
        )
        .add("Ui", "Ui", "界面")
        .add("Current Scale: ", "Current Scale: ", "当前缩放：")
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
        .add("Duplicate", "Duplicate", "复制")
        .add("Delete", "Delete", "删除")
        .add("Asset", "Asset", "资产")
        .add("Introduction", "Introduction", "介绍")
        .add("Asset System Introduction", "Asset System Introduction", "资产系统介绍")
        .add("Asset Introduction",
            "The Steel game engine has a simple asset system. Scenes, images, prefabs and other files used in the game are stored in the \"asset\" directory of the game project directory. Next to each asset file is an additional file with the same name but with the \".asset\" suffix added, which stores the ID of the asset. Each asset is loaded by its ID, not its file path. Therefore, if you need to move/rename an asset, you need to move/rename its corresponding \".asset\" file as well.",
            "Steel游戏引擎有一个简单的资产系统，场景、图像、预制件等游戏要用到的文件都保存在游戏项目目录的“asset”目录下。每个资产文件旁边都有一个同名的额外添加了“.asset”后缀的文件，这里面保存了这个资产的ID。每个资产是通过ID加载的，而不是其文件路径。因此如果你需要移动/重命名一个资产，你需要同时也移动/重命名其对应的“.asset”文件。",
        )
        .add("Select", "Select", "选择")
        .add("Save Prefab", "Save Prefab", "保存预制件")
        .add("Save As Prefab", "Save As Prefab", "另存为预制件")
        .add("New Entity", "New Entity", "新实体")
        .add("From Prefab", "From Prefab", "从预制件")
        .add("Create", "Create", "创建")
    }

    fn add(mut self, key: &'static str, eng: &'static str, chs: &'static str) -> Self {
        self.texts
            .entry(Language::Eng)
            .or_default()
            .insert(key, eng);
        self.texts
            .entry(Language::Chs)
            .or_default()
            .insert(key, chs);
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
