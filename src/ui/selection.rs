//! Логика выделения списка: одиночный клик, Ctrl, Shift, стрелки и рамка.
//!
//! Вынесено из виджетов отдельно, чтобы поведение можно было проверять
//! тестами без запуска интерфейса.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Состояние выделения.
#[derive(Debug, Default, Clone)]
pub struct State {
    paths: HashSet<PathBuf>,
    /// Точка отсчёта диапазона для Shift — остаётся на месте, пока
    /// пользователь расширяет выделение.
    anchor: Option<usize>,
    /// Текущий элемент: от него шагают стрелки.
    focus: Option<usize>,
}

impl State {
    pub fn contains(&self, path: &Path) -> bool {
        self.paths.contains(path)
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.paths.iter()
    }

    /// Точка отсчёта диапазона — проверяется тестами поведения Shift.
    #[allow(dead_code)]
    pub fn anchor(&self) -> Option<usize> {
        self.anchor
    }

    pub fn focus(&self) -> Option<usize> {
        self.focus
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self.anchor = None;
        self.focus = None;
    }

    /// Оставить только существующие пути (после обновления каталога).
    pub fn retain_existing(&mut self, alive: &HashSet<PathBuf>) {
        self.paths.retain(|path| alive.contains(path));
    }

    pub fn select_only(&mut self, path: PathBuf) {
        self.paths.clear();
        self.paths.insert(path);
    }

    pub fn select_all<I: IntoIterator<Item = PathBuf>>(&mut self, paths: I) {
        self.paths = paths.into_iter().collect();
    }

    /// Клик по элементу списка.
    ///
    /// * без модификаторов — выделяется только он;
    /// * `Ctrl` — переключается один элемент, точка отсчёта переносится;
    /// * `Shift` — выделяется диапазон от точки отсчёта, старое выделение
    ///   заменяется;
    /// * `Ctrl+Shift` — диапазон добавляется к уже выделенному.
    pub fn click<F>(&mut self, index: usize, ctrl: bool, shift: bool, path_at: F)
    where
        F: Fn(usize) -> Option<PathBuf>,
    {
        let Some(path) = path_at(index) else {
            return;
        };

        match (ctrl, shift) {
            (_, true) => {
                let from = self.anchor.unwrap_or(index);
                if !ctrl {
                    self.paths.clear();
                }
                self.add_range(from, index, &path_at);
                self.anchor = Some(from);
                self.focus = Some(index);
            }
            (true, false) => {
                if !self.paths.remove(&path) {
                    self.paths.insert(path);
                }
                self.anchor = Some(index);
                self.focus = Some(index);
            }
            (false, false) => {
                self.paths.clear();
                self.paths.insert(path);
                self.anchor = Some(index);
                self.focus = Some(index);
            }
        }
    }

    /// Перемещение стрелками: с Shift выделение растягивается от точки
    /// отсчёта, без него — переносится целиком.
    pub fn move_focus<F>(&mut self, target: usize, shift: bool, path_at: F)
    where
        F: Fn(usize) -> Option<PathBuf>,
    {
        let Some(path) = path_at(target) else {
            return;
        };

        if shift {
            let from = self.anchor.unwrap_or(target);
            self.paths.clear();
            self.add_range(from, target, &path_at);
            self.anchor = Some(from);
        } else {
            self.paths.clear();
            self.paths.insert(path);
            self.anchor = Some(target);
        }

        self.focus = Some(target);
    }

    /// Итог протяжки рамкой: `base` — что было выделено до начала (для
    /// протяжки с Ctrl), `hits` — попавшие в рамку элементы.
    pub fn apply_marquee<F>(&mut self, base: &HashSet<PathBuf>, hits: &[usize], path_at: F)
    where
        F: Fn(usize) -> Option<PathBuf>,
    {
        self.paths = base.clone();
        for &index in hits {
            if let Some(path) = path_at(index) {
                self.paths.insert(path);
            }
        }
    }

    fn add_range<F>(&mut self, from: usize, to: usize, path_at: &F)
    where
        F: Fn(usize) -> Option<PathBuf>,
    {
        let (start, end) = if from <= to { (from, to) } else { (to, from) };
        for index in start..=end {
            if let Some(path) = path_at(index) {
                self.paths.insert(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(count: usize) -> Vec<PathBuf> {
        (0..count).map(|i| PathBuf::from(format!("/dir/f{i}"))).collect()
    }

    fn lookup(items: &[PathBuf]) -> impl Fn(usize) -> Option<PathBuf> + '_ {
        move |index| items.get(index).cloned()
    }

    fn names(state: &State) -> Vec<String> {
        let mut out: Vec<String> = state
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        out.sort();
        out
    }

    #[test]
    fn обычный_клик_выделяет_один_элемент() {
        let items = items(5);
        let mut state = State::default();

        state.click(3, false, false, lookup(&items));
        assert_eq!(names(&state), ["f3"]);

        state.click(1, false, false, lookup(&items));
        assert_eq!(names(&state), ["f1"]);
        assert_eq!(state.anchor(), Some(1));
        assert_eq!(state.focus(), Some(1));
    }

    #[test]
    fn ctrl_переключает_элементы_по_одному() {
        let items = items(5);
        let mut state = State::default();

        state.click(0, false, false, lookup(&items));
        state.click(2, true, false, lookup(&items));
        state.click(4, true, false, lookup(&items));
        assert_eq!(names(&state), ["f0", "f2", "f4"]);

        // повторный Ctrl-клик снимает выделение с элемента
        state.click(2, true, false, lookup(&items));
        assert_eq!(names(&state), ["f0", "f4"]);
    }

    #[test]
    fn shift_выделяет_диапазон_от_точки_отсчёта() {
        let items = items(8);
        let mut state = State::default();

        state.click(2, false, false, lookup(&items));
        state.click(5, false, true, lookup(&items));
        assert_eq!(names(&state), ["f2", "f3", "f4", "f5"]);

        // точка отсчёта не двигается: диапазон можно ужать
        state.click(3, false, true, lookup(&items));
        assert_eq!(names(&state), ["f2", "f3"]);
        assert_eq!(state.anchor(), Some(2));
        assert_eq!(state.focus(), Some(3));
    }

    #[test]
    fn shift_работает_и_в_обратную_сторону() {
        let items = items(8);
        let mut state = State::default();

        state.click(5, false, false, lookup(&items));
        state.click(2, false, true, lookup(&items));
        assert_eq!(names(&state), ["f2", "f3", "f4", "f5"]);
    }

    #[test]
    fn shift_без_точки_отсчёта_выделяет_один_элемент() {
        let items = items(5);
        let mut state = State::default();

        state.click(3, false, true, lookup(&items));
        assert_eq!(names(&state), ["f3"]);
        assert_eq!(state.anchor(), Some(3));
    }

    #[test]
    fn ctrl_shift_добавляет_диапазон_к_выделенному() {
        let items = items(10);
        let mut state = State::default();

        state.click(0, false, false, lookup(&items));
        state.click(6, true, false, lookup(&items));
        state.click(8, true, true, lookup(&items));

        // f0 остаётся, к нему добавлен диапазон 6..8
        assert_eq!(names(&state), ["f0", "f6", "f7", "f8"]);
    }

    #[test]
    fn стрелки_переносят_выделение_а_с_shift_растягивают() {
        let items = items(6);
        let mut state = State::default();

        state.click(1, false, false, lookup(&items));
        state.move_focus(2, false, lookup(&items));
        assert_eq!(names(&state), ["f2"]);

        state.move_focus(4, true, lookup(&items));
        assert_eq!(names(&state), ["f2", "f3", "f4"]);
        assert_eq!(state.focus(), Some(4));

        // возврат к точке отсчёта сжимает диапазон обратно
        state.move_focus(3, true, lookup(&items));
        assert_eq!(names(&state), ["f2", "f3"]);
    }

    #[test]
    fn клик_мимо_списка_ничего_не_меняет() {
        let items = items(3);
        let mut state = State::default();

        state.click(1, false, false, lookup(&items));
        state.click(99, false, false, lookup(&items));
        assert_eq!(names(&state), ["f1"]);
    }

    #[test]
    fn рамка_заменяет_выделение_а_с_ctrl_дополняет() {
        let items = items(6);
        let mut state = State::default();

        state.apply_marquee(&HashSet::new(), &[1, 2], lookup(&items));
        assert_eq!(names(&state), ["f1", "f2"]);

        let base: HashSet<PathBuf> = [items[5].clone()].into_iter().collect();
        state.apply_marquee(&base, &[0], lookup(&items));
        assert_eq!(names(&state), ["f0", "f5"]);
    }
}
