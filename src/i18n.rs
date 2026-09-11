//! Small English/Korean/Japanese/Chinese lookup table.

#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    En,
    Ko,
    Ja,
    Zh,
}

impl Lang {
    /// Parse a language tag; falls back to English. Honours $ATK68_LANG / $LANG
    /// when given "auto".
    pub fn parse(s: &str) -> Lang {
        let s = if s.eq_ignore_ascii_case("auto") {
            std::env::var("ATK68_LANG")
                .or_else(|_| std::env::var("LANG"))
                .unwrap_or_default()
        } else {
            s.to_string()
        };
        match s.to_lowercase().get(..2).unwrap_or("") {
            "ko" => Lang::Ko,
            "ja" => Lang::Ja,
            "zh" => Lang::Zh,
            _ => Lang::En,
        }
    }

    fn idx(self) -> usize {
        self as usize
    }
}

/// key -> [English, Korean, Japanese, Chinese]
const STRINGS: &[(&str, [&str; 4])] = &[
    ("no_device", [
        "No ATK/VXE keyboard found (is it plugged in?)",
        "ATK/VXE 키보드를 찾을 수 없습니다 (연결되어 있나요?)",
        "ATK/VXEキーボードが見つかりません（接続されていますか？）",
        "未找到 ATK/VXE 键盘（是否已连接？）",
    ]),
    ("proto_version", ["Protocol version", "프로토콜 버전", "プロトコルバージョン", "协议版本"]),
    ("lighting", ["Lighting", "조명", "ライティング", "灯光"]),
    ("effect", ["Effect", "효과", "効果", "灯效"]),
    ("brightness", ["Brightness", "밝기", "明るさ", "亮度"]),
    ("speed", ["Speed", "속도", "速度", "速度"]),
    ("color", ["Color", "색상", "色", "颜色"]),
    ("rainbow", ["Rainbow", "무지개", "レインボー", "彩色"]),
    ("actuation", ["Actuation", "작동점", "アクチュエーション", "触发行程"]),
    ("rt_press", ["Rapid trigger (press)", "래피드 트리거(누름)", "ラピッドトリガー（押下）", "快速触发（按下）"]),
    ("rt_release", ["Rapid trigger (release)", "래피드 트리거(뗌)", "ラピッドトリガー（解放）", "快速触发（释放）"]),
    ("not_saved", [
        "Applied (RAM; reverts on replug — keep a profile with `export`/`apply`).",
        "적용됨 (RAM; 재연결 시 초기화 — `export`/`apply`로 프로파일 유지).",
        "適用（RAM；再接続でリセット — `export`/`apply`でプロファイル保持）。",
        "已应用（内存；重插会重置 — 用 `export`/`apply` 保留配置）。",
    ]),
];

/// Effect names indexed by effect id (the firmware's back-light `jRt` list,
/// extracted from the web bundle). Unknown ids render as `effect <n>`.
pub const EFFECTS: &[(&str, [&str; 4])] = &[
    ("spectrum", ["Spectrum Cycle", "스펙트럼 순환", "スペクトラムサイクル", "光谱循环"]),
    ("static", ["Static", "단색 고정", "静的", "常亮"]),
    ("wave_lr", ["Wave (L→R)", "물결(좌→우)", "ウェーブ(左→右)", "左往右冲浪"]),
    ("breathing", ["Breathing", "호흡", "ブリージング", "呼吸"]),
    ("snake", ["Snake", "뱀 흐름", "スネーク", "蛇形跑灯"]),
    ("reactive", ["Reactive", "키 반응", "リアクティブ", "磁力辐射"]),
    ("rotate", ["Rotate", "회전", "回転", "旋转跑灯"]),
    ("wave_center", ["Wave (to centre)", "물결(중앙)", "ウェーブ(中央)", "往中冲浪"]),
    ("fountain", ["Fountain", "분수", "ファウンテン", "七彩喷泉"]),
    ("laser", ["Laser", "레이저", "レーザー", "激光"]),
    ("custom", ["Custom", "사용자 지정", "カスタム", "自定义"]),
];

pub fn tr(lang: Lang, key: &'static str) -> &'static str {
    STRINGS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v[lang.idx()])
        .unwrap_or(key)
}

pub fn effect_name(lang: Lang, id: u8) -> String {
    EFFECTS
        .get(id as usize)
        .map(|(_, v)| v[lang.idx()].to_string())
        .unwrap_or_else(|| format!("effect {id}"))
}
