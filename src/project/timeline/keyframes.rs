/// キーフレームアニメーション機能の実装
///
/// このモジュールは、タイムライン上のクリップに対して時間経過によって
/// 変化するエフェクトやプロパティ値を設定するためのキーフレームアニメーション
/// 機能を提供します。
use std::collections::HashMap;
use thiserror::Error;

use crate::utility::time::{Duration, TimePosition};

/// キーフレームアニメーションに関するエラー
#[derive(Debug, Error)]
pub enum KeyframeError {
    /// 無効なキーフレーム時間
    #[error("無効なキーフレーム時間: {0}")]
    InvalidTime(TimePosition),

    /// 重複するキーフレーム
    #[error("この時間にはすでにキーフレームが存在します: {0}")]
    DuplicateKeyframe(TimePosition),

    /// 存在しないプロパティ
    #[error("プロパティが存在しません: {0}")]
    PropertyNotFound(String),

    /// キーフレームが見つからない
    #[error("キーフレームが見つかりません: property={property}, time={time}")]
    KeyframeNotFound {
        property: String,
        time: TimePosition,
    },

    /// その他のエラー
    #[error("キーフレームエラー: {0}")]
    Other(String),
}

/// キーフレーム操作の結果
pub type Result<T> = std::result::Result<T, KeyframeError>;

/// イージング関数の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingFunction {
    /// 線形補間（デフォルト）
    Linear,
    /// イーズイン（徐々に加速）
    EaseIn,
    /// イーズアウト（徐々に減速）
    EaseOut,
    /// イーズインアウト（加速してから減速）
    EaseInOut,
    /// ステップ（値の変化が段階的）
    Step,
}

impl EasingFunction {
    /// 2点間の補間値を計算
    ///
    /// # 引数
    ///
    /// * `t` - 0.0〜1.0の範囲の時間値
    /// * `start_value` - 開始値
    /// * `end_value` - 終了値
    ///
    /// # 戻り値
    ///
    /// 補間された値
    #[must_use]
    pub fn interpolate(&self, t: f64, start_value: f64, end_value: f64) -> f64 {
        let normalized_t = t.clamp(0.0, 1.0);
        let t_adjusted = match self {
            Self::Linear => normalized_t,
            Self::EaseIn => normalized_t * normalized_t,
            Self::EaseOut => normalized_t * (2.0 - normalized_t),
            Self::EaseInOut => {
                if normalized_t < 0.5 {
                    2.0 * normalized_t * normalized_t
                } else {
                    -1.0 + (4.0 - 2.0 * normalized_t) * normalized_t
                }
            }
            Self::Step => {
                if normalized_t < 1.0 {
                    0.0
                } else {
                    1.0
                }
            }
        };

        start_value + (end_value - start_value) * t_adjusted
    }

    /// イージング関数の名前を取得
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::EaseIn => "ease-in",
            Self::EaseOut => "ease-out",
            Self::EaseInOut => "ease-in-out",
            Self::Step => "step",
        }
    }
}

impl Default for EasingFunction {
    fn default() -> Self {
        Self::Linear
    }
}

/// 1つのキーフレームポイント
#[derive(Debug, Clone)]
pub struct KeyframePoint {
    /// キーフレームの時間位置
    time: TimePosition,
    /// キーフレームの値
    value: f64,
    /// イージング関数（次のキーフレームへの補間方法）
    easing: EasingFunction,
}

impl KeyframePoint {
    /// 新しいキーフレームポイントを作成
    ///
    /// # 引数
    ///
    /// * `time` - キーフレームの時間位置
    /// * `value` - キーフレームの値
    /// * `easing` - イージング関数
    ///
    /// # 戻り値
    ///
    /// 新しい`KeyframePoint`インスタンス
    #[must_use]
    pub fn new(time: TimePosition, value: f64, easing: EasingFunction) -> Self {
        Self {
            time,
            value,
            easing,
        }
    }

    /// キーフレームの時間位置を取得
    #[must_use]
    pub fn time(&self) -> TimePosition {
        self.time
    }

    /// キーフレームの値を取得
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// キーフレームのイージング関数を取得
    #[must_use]
    pub fn easing(&self) -> EasingFunction {
        self.easing
    }

    /// キーフレームの値を設定
    pub fn set_value(&mut self, value: f64) {
        self.value = value;
    }

    /// キーフレームのイージング関数を設定
    pub fn set_easing(&mut self, easing: EasingFunction) {
        self.easing = easing;
    }
}

/// 1つのプロパティに対するキーフレームのトラック
#[derive(Debug, Clone)]
pub struct KeyframeTrack {
    /// プロパティ名
    property_name: String,
    /// キーフレームのリスト（時間順にソート）
    keyframes: Vec<KeyframePoint>,
}

impl KeyframeTrack {
    /// 新しいキーフレームトラックを作成
    ///
    /// # 引数
    ///
    /// * `property_name` - プロパティの名前
    ///
    /// # 戻り値
    ///
    /// 新しい`KeyframeTrack`インスタンス
    #[must_use]
    pub fn new(property_name: String) -> Self {
        Self {
            property_name,
            keyframes: Vec::new(),
        }
    }

    /// プロパティ名を取得
    #[must_use]
    pub fn property_name(&self) -> &str {
        &self.property_name
    }

    /// キーフレームを追加
    ///
    /// # 引数
    ///
    /// * `time` - キーフレームの時間位置
    /// * `value` - キーフレームの値
    /// * `easing` - イージング関数
    ///
    /// # 戻り値
    ///
    /// 成功した場合は`Ok(())`、既に同じ時間にキーフレームが存在する場合はエラー
    ///
    /// # エラー
    ///
    /// 同じ時間位置に既にキーフレームが存在する場合は`KeyframeError::DuplicateKeyframe`を返す
    pub fn add_keyframe(
        &mut self,
        time: TimePosition,
        value: f64,
        easing: EasingFunction,
    ) -> Result<()> {
        // 同じ時間にキーフレームが存在するかチェック
        if self.keyframes.iter().any(|kf| kf.time() == time) {
            return Err(KeyframeError::DuplicateKeyframe(time));
        }

        // 新しいキーフレームを追加
        let keyframe = KeyframePoint::new(time, value, easing);
        self.keyframes.push(keyframe);

        // 時間でソート
        self.keyframes
            .sort_by(|a, b| a.time().partial_cmp(&b.time()).unwrap());

        Ok(())
    }

    /// キーフレームを更新
    ///
    /// # 引数
    ///
    /// * `time` - 更新するキーフレームの時間位置
    /// * `value` - 新しい値（Noneの場合は変更しない）
    /// * `easing` - 新しいイージング関数（Noneの場合は変更しない）
    ///
    /// # 戻り値
    ///
    /// 成功した場合は`Ok(())`、指定した時間にキーフレームが存在しない場合はエラー
    ///
    /// # エラー
    ///
    /// 指定した時間位置にキーフレームが存在しない場合は`KeyframeError::KeyframeNotFound`を返す
    pub fn update_keyframe(
        &mut self,
        time: TimePosition,
        value: Option<f64>,
        easing: Option<EasingFunction>,
    ) -> Result<()> {
        // キーフレームを検索
        if let Some(keyframe) = self.keyframes.iter_mut().find(|kf| kf.time() == time) {
            // 値を更新
            if let Some(value) = value {
                keyframe.set_value(value);
            }

            // イージングを更新
            if let Some(easing) = easing {
                keyframe.set_easing(easing);
            }

            Ok(())
        } else {
            Err(KeyframeError::KeyframeNotFound {
                property: self.property_name.clone(),
                time,
            })
        }
    }

    /// キーフレームを削除
    ///
    /// # 引数
    ///
    /// * `time` - 削除するキーフレームの時間位置
    ///
    /// # 戻り値
    ///
    /// 成功した場合は`Ok(())`、指定した時間にキーフレームが存在しない場合はエラー
    ///
    /// # エラー
    ///
    /// 指定した時間位置にキーフレームが存在しない場合は`KeyframeError::KeyframeNotFound`を返す
    pub fn remove_keyframe(&mut self, time: TimePosition) -> Result<()> {
        let initial_len = self.keyframes.len();
        self.keyframes.retain(|kf| kf.time() != time);

        if self.keyframes.len() == initial_len {
            Err(KeyframeError::KeyframeNotFound {
                property: self.property_name.clone(),
                time,
            })
        } else {
            Ok(())
        }
    }

    /// 特定の時間における値を取得
    ///
    /// # 引数
    ///
    /// * `time` - 値を取得する時間位置
    ///
    /// # 戻り値
    ///
    /// 時間位置における補間された値。キーフレームがない場合はNone
    #[must_use]
    pub fn get_value_at(&self, time: TimePosition) -> Option<f64> {
        if self.keyframes.is_empty() {
            return None;
        }

        // 時間が最初のキーフレームより前の場合
        if time <= self.keyframes[0].time() {
            return Some(self.keyframes[0].value());
        }

        // 時間が最後のキーフレームより後の場合
        let last_index = self.keyframes.len() - 1;
        if time >= self.keyframes[last_index].time() {
            return Some(self.keyframes[last_index].value());
        }

        // 時間が2つのキーフレームの間にある場合、補間する
        for i in 0..last_index {
            let current = &self.keyframes[i];
            let next = &self.keyframes[i + 1];

            if time >= current.time() && time < next.time() {
                // 2点間の時間の割合を計算
                let duration = (next.time() - current.time()).as_secs_f64();
                let elapsed = (time - current.time()).as_secs_f64();
                let t = elapsed / duration;

                // イージング関数を使って値を補間
                return Some(
                    current
                        .easing()
                        .interpolate(t, current.value(), next.value()),
                );
            }
        }

        // ここには到達しないはず
        None
    }

    /// キーフレームの数を取得
    #[must_use]
    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    /// キーフレームのリストを取得
    #[must_use]
    pub fn keyframes(&self) -> &[KeyframePoint] {
        &self.keyframes
    }

    /// キーフレームをすべて削除
    pub fn clear(&mut self) {
        self.keyframes.clear();
    }
}

/// 複数のプロパティに対するキーフレームアニメーション
#[derive(Debug, Clone)]
pub struct KeyframeAnimation {
    /// プロパティ名からキーフレームトラックへのマップ
    tracks: HashMap<String, KeyframeTrack>,
    /// アニメーションの合計時間
    duration: Duration,
}

impl KeyframeAnimation {
    /// 新しいキーフレームアニメーションを作成
    ///
    /// # 引数
    ///
    /// * `duration` - アニメーションの合計時間
    ///
    /// # 戻り値
    ///
    /// 新しい`KeyframeAnimation`インスタンス
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self {
            tracks: HashMap::new(),
            duration,
        }
    }

    /// アニメーションの合計時間を取得
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// アニメーションの合計時間を設定
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// キーフレームトラックを追加
    ///
    /// # 引数
    ///
    /// * `property` - プロパティ名
    /// * `track` - キーフレームトラック
    ///
    /// # 戻り値
    ///
    /// 成功した場合は`Ok(())`
    pub fn add_track(&mut self, property: &str, track: KeyframeTrack) -> Result<()> {
        self.tracks.insert(property.to_string(), track);
        Ok(())
    }

    /// キーフレームトラックを作成して追加（存在しない場合）
    ///
    /// # 引数
    ///
    /// * `property` - プロパティ名
    ///
    /// # 戻り値
    ///
    /// 成功した場合は`Ok(())`
    pub fn create_track_if_missing(&mut self, property: &str) -> Result<()> {
        if !self.tracks.contains_key(property) {
            let track = KeyframeTrack::new(property.to_string());
            self.tracks.insert(property.to_string(), track);
        }
        Ok(())
    }

    /// キーフレームトラックを取得
    ///
    /// # 引数
    ///
    /// * `property` - プロパティ名
    ///
    /// # 戻り値
    ///
    /// キーフレームトラックへの参照、またはプロパティが存在しない場合はNone
    #[must_use]
    pub fn get_track(&self, property: &str) -> Option<&KeyframeTrack> {
        self.tracks.get(property)
    }

    /// キーフレームトラックを可変で取得
    ///
    /// # 引数
    ///
    /// * `property` - プロパティ名
    ///
    /// # 戻り値
    ///
    /// キーフレームトラックへの可変参照、またはプロパティが存在しない場合はNone
    #[must_use]
    pub fn get_track_mut(&mut self, property: &str) -> Option<&mut KeyframeTrack> {
        self.tracks.get_mut(property)
    }

    /// キーフレームを追加
    ///
    /// # 引数
    ///
    /// * `property` - プロパティ名
    /// * `time` - キーフレームの時間位置
    /// * `value` - キーフレームの値
    /// * `easing` - イージング関数
    ///
    /// # 戻り値
    ///
    /// 成功した場合は`Ok(())`
    ///
    /// # エラー
    ///
    /// プロパティが存在しない場合は`KeyframeError::PropertyNotFound`を返す
    pub fn add_keyframe(
        &mut self,
        property: &str,
        time: TimePosition,
        value: f64,
        easing: EasingFunction,
    ) -> Result<()> {
        // 必要に応じてトラックを作成
        self.create_track_if_missing(property)?;

        // キーフレームを追加
        if let Some(track) = self.tracks.get_mut(property) {
            track.add_keyframe(time, value, easing)
        } else {
            Err(KeyframeError::PropertyNotFound(property.to_string()))
        }
    }

    /// 特定の時間におけるプロパティの値を取得
    ///
    /// # 引数
    ///
    /// * `property` - プロパティ名
    /// * `time` - 値を取得する時間位置
    ///
    /// # 戻り値
    ///
    /// 時間位置における補間された値。プロパティやキーフレームがない場合はNone
    #[must_use]
    pub fn get_value_at(&self, property: &str, time: TimePosition) -> Option<f64> {
        self.tracks
            .get(property)
            .and_then(|track| track.get_value_at(time))
    }

    /// プロパティを削除
    ///
    /// # 引数
    ///
    /// * `property` - プロパティ名
    ///
    /// # 戻り値
    ///
    /// 成功した場合は`Ok(())`
    ///
    /// # エラー
    ///
    /// プロパティが存在しない場合は`KeyframeError::PropertyNotFound`を返す
    pub fn remove_property(&mut self, property: &str) -> Result<()> {
        if self.tracks.remove(property).is_some() {
            Ok(())
        } else {
            Err(KeyframeError::PropertyNotFound(property.to_string()))
        }
    }

    /// プロパティのリストを取得
    #[must_use]
    pub fn properties(&self) -> Vec<String> {
        self.tracks.keys().cloned().collect()
    }

    /// キーフレームアニメーションをクリア
    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    /// 特定のプロパティに対するキーフレームトラックを追加または取得
    ///
    /// プロパティが存在しない場合は新しいトラックを作成します。
    ///
    /// # Arguments
    ///
    /// * `property` - プロパティ名
    ///
    /// # Returns
    ///
    /// `&mut KeyframeTrack` - キーフレームトラックへの可変参照
    pub fn get_or_create_track(&mut self, property: &str) -> &mut KeyframeTrack {
        self.tracks
            .entry(property.to_string())
            .or_insert_with(|| KeyframeTrack::new(property.to_string()))
    }

    /// キーフレームを削除
    ///
    /// # Arguments
    ///
    /// * `property` - プロパティ名
    /// * `time` - キーフレームの時間位置
    ///
    /// # Returns
    ///
    /// `Result<(), KeyframeError>` - 成功した場合はOk、失敗した場合はエラー
    pub fn remove_keyframe(&mut self, property: &str, time: TimePosition) -> Result<()> {
        if let Some(track) = self.get_track_mut(property) {
            track.remove_keyframe(time)
        } else {
            Err(KeyframeError::PropertyNotFound(property.to_string()))
        }
    }

    /// 特定のプロパティのキーフレームトラックが存在するかどうかを確認
    ///
    /// # Arguments
    ///
    /// * `property` - 確認するプロパティ名
    ///
    /// # Returns
    ///
    /// `bool` - プロパティのキーフレームトラックが存在する場合はtrue、それ以外はfalse
    #[must_use]
    pub fn has_property(&self, property: &str) -> bool {
        self.tracks.contains_key(property)
    }

    /// すべてのキーフレームトラックを取得
    #[must_use]
    pub fn tracks(&self) -> &HashMap<String, KeyframeTrack> {
        &self.tracks
    }

    /// すべてのプロパティ名を取得
    #[must_use]
    pub fn property_names(&self) -> Vec<&str> {
        self.tracks.keys().map(|k| k.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    // TimePositionを秒数から作成するヘルパー関数
    fn time_pos(seconds: f64) -> TimePosition {
        // 負の値はTimePosition::from_secondsを使用して直接変換
        TimePosition::from_seconds(seconds)
    }

    #[test]
    fn test_easing_functions() {
        // 線形補間
        assert_eq!(EasingFunction::Linear.interpolate(0.5, 0.0, 10.0), 5.0);

        // イーズイン補間（加速）
        let ease_in = EasingFunction::EaseIn.interpolate(0.5, 0.0, 10.0);
        assert!(ease_in < 5.0); // 加速するので中間点では線形より小さい値

        // イーズアウト補間（減速）
        let ease_out = EasingFunction::EaseOut.interpolate(0.5, 0.0, 10.0);
        assert!(ease_out > 5.0); // 減速するので中間点では線形より大きい値

        // 範囲外の時間値は0.0-1.0に制限される
        assert_eq!(EasingFunction::Linear.interpolate(-0.5, 0.0, 10.0), 0.0);
        assert_eq!(EasingFunction::Linear.interpolate(1.5, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_keyframe_track() {
        let mut track = KeyframeTrack::new("opacity".to_string());

        // キーフレーム追加
        track
            .add_keyframe(time_pos(0.0), 0.0, EasingFunction::Linear)
            .unwrap();
        track
            .add_keyframe(time_pos(10.0), 1.0, EasingFunction::EaseOut)
            .unwrap();

        // 同じ時間に追加するとエラー
        assert!(
            track
                .add_keyframe(time_pos(0.0), 0.5, EasingFunction::Linear)
                .is_err()
        );

        // 値の補間
        assert_eq!(track.get_value_at(time_pos(0.0)), Some(0.0)); // 開始点
        assert_eq!(track.get_value_at(time_pos(10.0)), Some(1.0)); // 終了点

        // 中間点（Linear補間なら0.5、しかし2つ目のキーフレームはEaseOutなので）
        let mid_value = track.get_value_at(time_pos(5.0)).unwrap();
        assert!(mid_value >= 0.5); // EaseOutなので中間点では0.5以上の値

        // 範囲外
        assert_eq!(track.get_value_at(time_pos(-1.0)), Some(0.0)); // 範囲前は最初の値
        assert_eq!(track.get_value_at(time_pos(11.0)), Some(1.0)); // 範囲後は最後の値

        // キーフレームの更新
        track
            .update_keyframe(time_pos(0.0), Some(0.2), None)
            .unwrap();
        assert_eq!(track.get_value_at(time_pos(0.0)), Some(0.2)); // 値が更新された

        // キーフレームの削除
        track.remove_keyframe(time_pos(0.0)).unwrap();
        assert_eq!(track.keyframe_count(), 1); // 1つだけ残る
    }

    #[test]
    fn test_keyframe_animation() {
        let mut animation = KeyframeAnimation::new(Duration::from_seconds(10.0));

        // プロパティ追加
        animation
            .add_keyframe("opacity", time_pos(0.0), 0.0, EasingFunction::Linear)
            .unwrap();
        animation
            .add_keyframe("opacity", time_pos(10.0), 1.0, EasingFunction::EaseOut)
            .unwrap();

        animation
            .add_keyframe("scale", time_pos(0.0), 1.0, EasingFunction::Linear)
            .unwrap();
        animation
            .add_keyframe("scale", time_pos(5.0), 1.5, EasingFunction::EaseIn)
            .unwrap();
        animation
            .add_keyframe("scale", time_pos(10.0), 2.0, EasingFunction::Linear)
            .unwrap();

        // 値の取得
        assert_eq!(animation.get_value_at("opacity", time_pos(0.0)), Some(0.0));
        assert_eq!(animation.get_value_at("scale", time_pos(0.0)), Some(1.0));

        // 存在しないプロパティ
        assert_eq!(animation.get_value_at("rotation", time_pos(5.0)), None);

        // プロパティリスト
        let properties = animation.properties();
        assert_eq!(properties.len(), 2);
        assert!(properties.contains(&"opacity".to_string()));
        assert!(properties.contains(&"scale".to_string()));
    }
}
