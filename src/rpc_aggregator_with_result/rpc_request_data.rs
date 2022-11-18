use std::sync::Arc;

use crate::TaskCompletion;

use super::rcp_aggregator_with_result_inner::Request;

pub struct RcpRequestData<
    TItem: Send + Sync + 'static,
    TResult: Send + Sync + 'static,
    TError: Send + Sync + 'static,
> {
    data: Option<Vec<TItem>>,
    completions: Vec<TaskCompletion<TResult, Arc<TError>>>,
}

impl<
        TItem: Send + Sync + 'static,
        TResult: Send + Sync + 'static,
        TError: Send + Sync + 'static,
    > RcpRequestData<TItem, TResult, TError>
{
    pub fn new(requests: Vec<Request<TItem, TResult, TError>>) -> Self {
        let mut data = Vec::with_capacity(requests.len());
        let mut completions = Vec::with_capacity(requests.len());

        for request in requests {
            data.push(request.request_data);
            completions.push(request.completion);
        }

        Self {
            data: Some(data),
            completions,
        }
    }

    pub fn get_data_to_callback(&mut self) -> Arc<Vec<TItem>> {
        let mut new_result = None;
        std::mem::swap(&mut new_result, &mut self.data);
        Arc::new(new_result.unwrap())
    }

    pub fn set_results(&mut self, mut results: Vec<TResult>) -> Result<(), String> {
        if results.len() != self.completions.len() {
            return Err(format!(
                "amount of results {} != amount of requests {}",
                results.len(),
                self.completions.len()
            ));
        }

        for completion in &mut self.completions {
            let result = results.remove(0);
            if let Err(err) = completion.try_set_ok(result) {
                println!("can not set result: {:?}", err);
            }
        }

        Ok(())
    }

    pub fn set_panic(mut self, message: &str) {
        for completion in &mut self.completions {
            if let Err(err) = completion.try_set_panic(message.to_string()) {
                println!("Can not set panic result to the task completion. {:?}", err);
            }
        }
    }

    pub fn set_error(mut self, err: TError) {
        let err = Arc::new(err);
        for completion in &mut self.completions {
            if let Err(err) = completion.try_set_error(err.clone()) {
                println!("set_error: {:?}", err);
            }
        }
    }
}
