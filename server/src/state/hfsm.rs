#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Initialize,
    DocumentChanged,
    AnalysisCompleted,
    TriggerAutoInjection,
    LockAcquired,
    LockFailed,
    WriteCompleted,
    WriteError,
    LockReleased,
    RecoveryCompleted,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadyState {
    DocumentIdle,
    Analyzing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LockingWriteState {
    AcquiringLock,
    Writing,
    ReleasingLock,
    ConflictHandling,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InitializedState {
    Ready(ReadyState),
    LockingWrite(LockingWriteState),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerState {
    Uninitialized,
    Initialized(InitializedState),
}

pub struct Hfsm {
    state: ServerState,
}

impl Hfsm {
    pub fn new() -> Self {
        Self {
            state: ServerState::Uninitialized,
        }
    }

    pub fn get_state(&self) -> &ServerState {
        &self.state
    }

    pub fn dispatch(&mut self, event: Event) {
        self.state = match (&self.state, event) {
            // Uninitialized からの遷移
            (ServerState::Uninitialized, Event::Initialize) => {
                ServerState::Initialized(InitializedState::Ready(ReadyState::DocumentIdle))
            }
            (ServerState::Uninitialized, _) => {
                // 初期化前は他のイベントはすべて無視
                ServerState::Uninitialized
            }

            // Initialized状態での遷移
            (ServerState::Initialized(init_state), event) => {
                match (init_state, event) {
                    // シャットダウンはどの状態からでも遷移可能
                    (_, Event::Shutdown) => ServerState::Uninitialized,

                    // Ready 状態の中での遷移
                    (InitializedState::Ready(ready_state), event) => {
                        match (ready_state, event) {
                            (ReadyState::DocumentIdle, Event::DocumentChanged) => {
                                ServerState::Initialized(InitializedState::Ready(ReadyState::Analyzing))
                            }
                            (ReadyState::Analyzing, Event::AnalysisCompleted) => {
                                ServerState::Initialized(InitializedState::Ready(ReadyState::DocumentIdle))
                            }
                            // 解析完了後、自動インジェクションが必要な場合の遷移
                            (ReadyState::DocumentIdle, Event::TriggerAutoInjection) |
                            (ReadyState::Analyzing, Event::TriggerAutoInjection) => {
                                ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::AcquiringLock))
                            }
                            // その他のイベントは現在の状態を維持
                            (current, _) => ServerState::Initialized(InitializedState::Ready(current.clone())),
                        }
                    }

                    // LockingWrite 状態の中での遷移
                    (InitializedState::LockingWrite(locking_state), event) => {
                        match (locking_state, event) {
                            (LockingWriteState::AcquiringLock, Event::LockAcquired) => {
                                ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::Writing))
                            }
                            (LockingWriteState::AcquiringLock, Event::LockFailed) => {
                                ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::ConflictHandling))
                            }
                            (LockingWriteState::Writing, Event::WriteCompleted) => {
                                ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::ReleasingLock))
                            }
                            (LockingWriteState::Writing, Event::WriteError) => {
                                ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::ConflictHandling))
                            }
                            (LockingWriteState::ReleasingLock, Event::LockReleased) => {
                                ServerState::Initialized(InitializedState::Ready(ReadyState::DocumentIdle))
                            }
                            (LockingWriteState::ConflictHandling, Event::RecoveryCompleted) => {
                                ServerState::Initialized(InitializedState::Ready(ReadyState::DocumentIdle))
                            }
                            // 書き込みロック中は、DocumentChanged などの外部編集イベントは無視する
                            (current, Event::DocumentChanged) => {
                                ServerState::Initialized(InitializedState::LockingWrite(current.clone()))
                            }
                            // その他の無効なイベントは状態を維持
                            (current, _) => ServerState::Initialized(InitializedState::LockingWrite(current.clone())),
                        }
                    }
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hfsm_transitions() {
        let mut hfsm = Hfsm::new();
        assert_eq!(*hfsm.get_state(), ServerState::Uninitialized);

        // 未初期化状態での無効なイベントは無視されること
        hfsm.dispatch(Event::DocumentChanged);
        assert_eq!(*hfsm.get_state(), ServerState::Uninitialized);

        // 初期化
        hfsm.dispatch(Event::Initialize);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::Ready(ReadyState::DocumentIdle))
        );

        // ドキュメント変更検知
        hfsm.dispatch(Event::DocumentChanged);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::Ready(ReadyState::Analyzing))
        );

        // 解析完了
        hfsm.dispatch(Event::AnalysisCompleted);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::Ready(ReadyState::DocumentIdle))
        );

        // 自動インジェクション開始
        hfsm.dispatch(Event::TriggerAutoInjection);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::AcquiringLock))
        );

        // ロック取得中のドキュメント変更は無視されること
        hfsm.dispatch(Event::DocumentChanged);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::AcquiringLock))
        );

        // ロック取得成功
        hfsm.dispatch(Event::LockAcquired);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::Writing))
        );

        // 書き込み完了
        hfsm.dispatch(Event::WriteCompleted);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::ReleasingLock))
        );

        // ロック解放
        hfsm.dispatch(Event::LockReleased);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::Ready(ReadyState::DocumentIdle))
        );

        // シャットダウン
        hfsm.dispatch(Event::Shutdown);
        assert_eq!(*hfsm.get_state(), ServerState::Uninitialized);
    }

    #[test]
    fn test_hfsm_conflict_handling() {
        let mut hfsm = Hfsm::new();
        hfsm.dispatch(Event::Initialize);
        hfsm.dispatch(Event::TriggerAutoInjection);
        
        // ロック失敗
        hfsm.dispatch(Event::LockFailed);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::LockingWrite(LockingWriteState::ConflictHandling))
        );

        // 復旧完了
        hfsm.dispatch(Event::RecoveryCompleted);
        assert_eq!(
            *hfsm.get_state(),
            ServerState::Initialized(InitializedState::Ready(ReadyState::DocumentIdle))
        );
    }
}
