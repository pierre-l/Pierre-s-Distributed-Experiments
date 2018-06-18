use futures::{self, Poll, Async};
use futures::{Stream, Sink};

/// A combinator used to flatten a stream-of-streams into one long stream of
/// elements.
/// This differs from tokio's flatten implementation in that it polls the
/// streams in a round-robin semi-concurrent fashion whereas tokio's implementation
/// will poll the same stream as long as it is ready. The only way the FlattenSelect
/// will poll twice the same stream in a row is if it is the only one that is ready
/// at that time.
///
/// This implementation has room for improvement, especially performance-wise.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct FlattenSelect<S>
    where S: Stream,
{
    stream: S,
    still_has_children: bool,
    children: Vec<S::Item>,
    last_polled_index: usize,
}

pub fn new<S>(s: S) -> FlattenSelect<S>
    where S: Stream,
          S::Item: Stream,
          <S::Item as Stream>::Error: From<S::Error>,
{
    FlattenSelect {
        stream: s,
        still_has_children: true,
        children: vec![],
        last_polled_index: 0,
    }
}

// Directly copied from tokio's flatten implementation.
// Required in cases like chaining stream operators
#[allow(dead_code)]
impl<S: Stream> FlattenSelect<S> {
    /// Acquires a reference to the underlying stream that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// stream which may otherwise confuse this combinator.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// Consumes this combinator, returning the underlying stream.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> S {
        self.stream
    }
}

// Directly copied from tokio's flatten implementation.
// Forwarding impl of Sink from the underlying stream
impl<S> Sink for FlattenSelect<S>
    where S: Sink + Stream
{
    type SinkItem = S::SinkItem;
    type SinkError = S::SinkError;

    fn start_send(&mut self, item: S::SinkItem) -> futures::StartSend<S::SinkItem, S::SinkError> {
        self.stream.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), S::SinkError> {
        self.stream.poll_complete()
    }

    fn close(&mut self) -> Poll<(), S::SinkError> {
        self.stream.close()
    }
}

impl<S> Stream for FlattenSelect<S>
    where S: Stream,
          S::Item: Stream,
          <S::Item as Stream>::Error: From<S::Error>,
{
    type Item = <S::Item as Stream>::Item;
    type Error = <S::Item as Stream>::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if self.still_has_children{
            match self.stream.poll() {
                Ok(Async::Ready(Some(e))) => {
                    self.children.push(e);
                },
                Ok(Async::Ready(None)) => {
                    self.still_has_children = false;
                },
                Err(err) => {
                    return Err(From::from(err));
                }
                _other => {},
            }
        }

        let children_len = self.children.len();

        if !self.still_has_children && children_len == 0 {
            return Ok(Async::Ready(None));
        } else if children_len > 0 {
            let range_start = self.last_polled_index +1;
            let range_end = range_start + children_len -1;

            let mut to_remove = vec![];
            for index in range_start..range_end{
                let index = index % children_len;
                self.last_polled_index = index;

                let mut child = &mut self.children[index];

                match child.poll() {
                    Ok(Async::Ready(None)) => {
                        to_remove.push(index);
                    },
                    Ok(Async::Ready(Some(item))) => {
                        self.last_polled_index = index;
                        return Ok(Async::Ready(Some(item)));
                    }
                    Err(err) => {
                        return Err(err);
                    }
                    _other => {},
                }
            }

            // Remove the items from the highest index to the lowest. This avoids re-adjusting the
            // indexes of the item to remove at every iteration. Leads to O(n*log n) in the worst case
            // instead of O(n^2)
            to_remove.sort();
            let _: () = to_remove.iter().rev()
                .map(|index_to_remove|{
                    if self.last_polled_index > *index_to_remove {
                        self.last_polled_index -= 1;
                    }

                    self.children.remove(*index_to_remove);
                })
                .collect()
            ;
        }

        Ok(Async::NotReady) // No child was ready, consider this stream "not ready".
    }
}
