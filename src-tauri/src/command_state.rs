//! Borrowed service dependencies shared by desktop commands and the LAN server.
//! Tauri injects these locally; the server supplies its database and connection's session.
pub(crate) struct State<'a, T: Send + Sync + 'static>(pub &'a T);

impl<T: Send + Sync + 'static> std::ops::Deref for State<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

impl<'a, 'de: 'a, T: Send + Sync + 'static, R: tauri::Runtime> tauri::ipc::CommandArg<'de, R>
    for State<'a, T>
{
    fn from_command(
        command: tauri::ipc::CommandItem<'de, R>,
    ) -> Result<Self, tauri::ipc::InvokeError> {
        let value = <tauri::State<'a, T> as tauri::ipc::CommandArg<'de, R>>::from_command(command)?;
        Ok(Self(value.inner()))
    }
}
