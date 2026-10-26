//! Text shown by the CLI. Every message has a translation for each language.

use crate::tweaks::ToggleId;
use anyhow::{bail, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Ko,
    Ja,
    Zh,
}

#[derive(Clone, Copy)]
pub enum Text {
    NoDevice,
    Firmware,
    Profile,
    ReportRate,
    Index,
    Lighting,
    Effect,
    Brightness,
    Speed,
    Color,
    Rainbow,
    Toggles,
    Actuation,
    RtPress,
    RtRelease,
    AppliedRam,
    ExportedProfile,
    AppliedProfile,
    SavedFlash,
    ResetFlash,
    KeyRt,
    On,
    Off,
}

impl Lang {
    /// Explicit choices are checked; `auto` uses ATK68_LANG, then LANG.
    pub fn parse(choice: &str) -> Result<Self> {
        match choice.to_ascii_lowercase().as_str() {
            "auto" => {
                let locale = std::env::var("ATK68_LANG")
                    .ok()
                    .filter(|value| !value.is_empty())
                    .or_else(|| std::env::var("LANG").ok())
                    .unwrap_or_default();
                Ok(Self::from_locale(&locale))
            }
            "en" => Ok(Self::En),
            "ko" => Ok(Self::Ko),
            "ja" => Ok(Self::Ja),
            "zh" => Ok(Self::Zh),
            _ => bail!("unsupported language '{choice}' (use en, ko, ja, zh, or auto)"),
        }
    }

    fn from_locale(locale: &str) -> Self {
        let code = locale.split(['_', '-', '.', '@']).next().unwrap_or("");
        match code.to_ascii_lowercase().as_str() {
            "ko" => Self::Ko,
            "ja" => Self::Ja,
            "zh" => Self::Zh,
            _ => Self::En,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::En => 0,
            Self::Ko => 1,
            Self::Ja => 2,
            Self::Zh => 3,
        }
    }

    pub fn text(self, key: Text) -> &'static str {
        let row = match key {
            Text::NoDevice => [
                "No ATK/VXE keyboard found.",
                "ATK/VXE 키보드를 찾을 수 없습니다.",
                "ATK/VXEキーボードが見つかりません。",
                "未找到 ATK/VXE 键盘。",
            ],
            Text::Firmware => ["firmware", "펌웨어", "ファームウェア", "固件"],
            Text::Profile => ["profile", "프로파일", "プロファイル", "配置文件"],
            Text::ReportRate => ["report rate", "보고율", "ポーリングレート", "回报率"],
            Text::Index => ["index", "인덱스", "インデックス", "索引"],
            Text::Lighting => ["Lighting", "조명", "ライティング", "灯光"],
            Text::Effect => ["Effect", "효과", "効果", "灯效"],
            Text::Brightness => ["Brightness", "밝기", "明るさ", "亮度"],
            Text::Speed => ["Speed", "속도", "速度", "速度"],
            Text::Color => ["Color", "색상", "色", "颜色"],
            Text::Rainbow => ["Rainbow", "무지개", "レインボー", "彩色"],
            Text::Toggles => ["Toggles", "토글", "トグル", "开关设置"],
            Text::Actuation => ["Actuation", "작동점", "アクチュエーション", "触发行程"],
            Text::RtPress => [
                "Rapid trigger (press)",
                "래피드 트리거(누름)",
                "ラピッドトリガー（押下）",
                "快速触发（按下）",
            ],
            Text::RtRelease => [
                "Rapid trigger (release)",
                "래피드 트리거(뗌)",
                "ラピッドトリガー（解放）",
                "快速触发（释放）",
            ],
            Text::AppliedRam => [
                "Applied (RAM; reverts on replug — keep a profile with `export`/`apply`).",
                "적용됨 (RAM; 재연결 시 초기화 — `export`/`apply`로 프로파일 유지).",
                "適用（RAM；再接続でリセット — `export`/`apply`でプロファイル保持）。",
                "已应用（内存；重插会重置 — 用 `export`/`apply` 保留配置）。",
            ],
            Text::ExportedProfile => [
                "exported profile to",
                "프로파일 내보냄:",
                "プロファイルを書き出しました:",
                "已导出配置文件:",
            ],
            Text::AppliedProfile => ["applied", "적용됨:", "適用しました:", "已应用:"],
            Text::SavedFlash => [
                "saved settings to flash",
                "설정을 플래시에 저장함",
                "設定をフラッシュに保存しました",
                "已将设置保存到闪存",
            ],
            Text::ResetFlash => [
                "reset settings in flash",
                "플래시 설정을 초기화함",
                "フラッシュの設定をリセットしました",
                "已重置闪存中的设置",
            ],
            Text::KeyRt => [
                "key rapid trigger",
                "키 래피드 트리거",
                "キーのラピッドトリガー",
                "单键快速触发",
            ],
            Text::On => ["on", "켜짐", "オン", "开启"],
            Text::Off => ["off", "꺼짐", "オフ", "关闭"],
        };
        row[self.index()]
    }

    pub fn bool_value(self, value: bool) -> &'static str {
        self.text(if value { Text::On } else { Text::Off })
    }

    pub fn effect_name(self, id: u8) -> String {
        EFFECTS
            .get(id as usize)
            .map(|row| row[self.index()].to_string())
            .unwrap_or_else(|| format!("{} {id}", self.text(Text::Effect)))
    }

    pub fn toggle_help(self, id: ToggleId) -> &'static str {
        let row = match id {
            ToggleId::Winlock => [
                "disable the Windows key",
                "Windows 키 비활성화",
                "Windowsキーを無効化",
                "禁用 Windows 键",
            ],
            ToggleId::Wasd => [
                "swap WASD with arrow keys",
                "WASD와 방향키 교환",
                "WASDと矢印キーを交換",
                "交换 WASD 与方向键",
            ],
            ToggleId::Mac => [
                "Mac layout (0=Win, 1=Mac)",
                "Mac 배열 (0=Win, 1=Mac)",
                "Mac配列（0=Win、1=Mac）",
                "Mac 布局（0=Win，1=Mac）",
            ],
            ToggleId::Rt => [
                "global rapid trigger",
                "전체 래피드 트리거",
                "全体のラピッドトリガー",
                "全局快速触发",
            ],
            ToggleId::LowLatency => [
                "low-latency mode",
                "저지연 모드",
                "低遅延モード",
                "低延迟模式",
            ],
            ToggleId::BottomTrigger => [
                "full-press bottom trigger",
                "완전 누름 바닥 트리거",
                "底打ちトリガー",
                "触底触发",
            ],
            ToggleId::Debounce => [
                "key debounce (raw units)",
                "키 디바운스 (원시 단위)",
                "デバウンス（生値）",
                "按键消抖（原始单位）",
            ],
        };
        row[self.index()]
    }
}

/// Firmware effect IDs in order.
const EFFECTS: [[&str; 4]; 11] = [
    [
        "Spectrum Cycle",
        "스펙트럼 순환",
        "スペクトラムサイクル",
        "光谱循环",
    ],
    ["Static", "단색 고정", "静的", "常亮"],
    ["Wave (L→R)", "물결(좌→우)", "ウェーブ(左→右)", "左往右冲浪"],
    ["Breathing", "호흡", "ブリージング", "呼吸"],
    ["Snake", "뱀 흐름", "スネーク", "蛇形跑灯"],
    ["Reactive", "키 반응", "リアクティブ", "磁力辐射"],
    ["Rotate", "회전", "回転", "旋转跑灯"],
    [
        "Wave (to centre)",
        "물결(중앙)",
        "ウェーブ(中央)",
        "往中冲浪",
    ],
    ["Fountain", "분수", "ファウンテン", "七彩喷泉"],
    ["Laser", "레이저", "レーザー", "激光"],
    ["Custom", "사용자 지정", "カスタム", "自定义"],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_language_is_checked() {
        assert_eq!(Lang::parse("KO").unwrap(), Lang::Ko);
        assert!(Lang::parse("kr").is_err());
    }

    #[test]
    fn locale_suffixes_are_ignored() {
        assert_eq!(Lang::from_locale("ko_KR.UTF-8"), Lang::Ko);
        assert_eq!(Lang::from_locale("ja-JP"), Lang::Ja);
        assert_eq!(Lang::from_locale("C.UTF-8"), Lang::En);
    }

    #[test]
    fn unknown_effect_uses_translated_label() {
        assert_eq!(Lang::Ko.effect_name(99), "효과 99");
    }
}
