
#[macro_export]
macro_rules! command {
    (
        $(
           $(#[$docs:meta])*
           $vis:vis $name:ident($($param:ident: $input:ty),*) $(-> $output:ty)?;
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
            $vis async fn $name (&self, $($param: $input,)*) $(-> $output)? {
                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                let data = Command::$name{$($param,)* resp_tx};
                self.tx.send(data).await.unwrap();
                resp_rx.await.unwrap()
            }
        )+
        }
        pub struct SpawnCommandSender {
            tx: tokio::sync::mpsc::Sender<Command>,
        }
        
        impl SpawnCommandSender {
        $(
            $vis fn $name (self, $($param: $input,)*) {
                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                let data = Command::$name{$($param,)* resp_tx};
                let tx = self.tx;
                tokio::spawn(async move{
                    tx.send(data).await.unwrap();
                    let _ = resp_rx.await.unwrap();
                });
            }
        )+
        }
        
        impl CommandSender {
            pub fn spawn(&self) -> SpawnCommandSender {
                SpawnCommandSender {tx: self.tx.clone() }
            }
        }
    };
}
