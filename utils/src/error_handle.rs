#[macro_export]
macro_rules! command_handler_wrapper {
    ($command:expr) => {
        $command.await.map_err(|err| {
            let err = dbg!(err)
                .chain()
                .rev()
                .enumerate()
                .map(|(i, e)| match i {
                    0 => format!("Got an error: {e}"),
                    _ => format!("While {e}"),
                })
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::anyhow!("{err}")
        })
    };
}
// pub async fn command_handler_wrapper<
//     Command: Fn(poise::Context<'_, Data, anyhow::Error>) -> CommandResult,
//     CommandResult: Future<Output = anyhow::Result<T>>,
//     T,
//     Data,
// >(
//     ctx: poise::Context<'_, Data, anyhow::Error>,
//     command: Command,
// ) -> anyhow::Result<T> {
//     command(ctx).await.map_err(|err| {
//         let err = err
//             .chain()
//             .rev()
//             .enumerate()
//             .map(|(i, e)| match i {
//                 0 => format!("Got an error: {e}"),
//                 _ => format!("While {e}"),
//             })
//             .collect::<Vec<_>>()
//             .join("\n");
//         anyhow::anyhow!("{err}")
//     })
// }
