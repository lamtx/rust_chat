#[macro_export]
macro_rules! command {
    (
        $(
           $(#[$docs:meta])*
           $name:ident($($param:ident: $input:ty),*) $(-> $output:ty)?,
        )+
    ) => {
        #[warn(unused_parens)]
        pub enum Command {
        $(
            $(#[$docs])*
            $name {
                $($param: $input,)*
                resp_tx: tokio::sync::oneshot::Sender<($($output)?)>,
            },
        )+
        } 
        #[derive(Clone)]
        pub struct CommandSender {
            tx: tokio::sync::mpsc::Sender<Command>,
        }
        impl CommandSender {
        $(
            pub async fn $name (&self, $($param: $input,)*) $(-> $output)? {
                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                let data = Command::$name{$($param,)* resp_tx};
                self.tx.send(data).await.unwrap();
                resp_rx.await.unwrap()
            }
        )+
        }
    };
}
