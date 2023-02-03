#![allow(clippy::needless_lifetimes)]

use async_graphql::{
    connection::{query, Connection, Edge},
    Context, Enum, Error, Interface, Object, OutputType, Result,
};

use super::StarWars;
use crate::starwars::StarWarsChar;

/// One of the films in the Star Wars Trilogy
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum Episode {
    /// Released in 1977.
    NewHope,

    /// Released in 1980.
    Empire,

    /// Released in 1983.
    Jedi,
}

pub struct Human<'a>(&'a StarWarsChar);

/// A humanoid creature in the Star Wars universe.
#[Object]
impl<'a> Human<'a> {
    /// The id of the human.
    async fn id(&self) -> &str {
        self.0.id
    }

    /// The name of the human.
    async fn name(&self) -> &str {
        self.0.name
    }

    /// The friends of the human, or an empty list if they have none.
    async fn friends<'ctx>(&self, ctx: &Context<'ctx>) -> Vec<Character<'ctx>> {
        let star_wars = ctx.data_unchecked::<StarWars>();
        star_wars
            .friends(self.0)
            .into_iter()
            .map(|ch| {
                if ch.is_human {
                    Human(ch).into()
                } else {
                    Droid(ch).into()
                }
            })
            .collect()
    }

    /// Which movies they appear in.
    async fn appears_in(&self) -> &[Episode] {
        &self.0.appears_in
    }

    /// The home planet of the human, or null if unknown.
    async fn home_planet(&self) -> &Option<&str> {
        &self.0.home_planet
    }
}

pub struct Droid<'a>(&'a StarWarsChar);

/// A mechanical creature in the Star Wars universe.
#[Object]
impl<'a> Droid<'a> {
    /// The id of the droid.
    async fn id(&self) -> &str {
        self.0.id
    }

    /// The name of the droid.
    async fn name(&self) -> &str {
        self.0.name
    }

    /// The friends of the droid, or an empty list if they have none.
    async fn friends<'ctx>(&self, ctx: &Context<'ctx>) -> Vec<Character<'ctx>> {
        let star_wars = ctx.data_unchecked::<StarWars>();
        star_wars
            .friends(self.0)
            .into_iter()
            .map(|ch| {
                if ch.is_human {
                    Human(ch).into()
                } else {
                    Droid(ch).into()
                }
            })
            .collect()
    }

    /// Which movies they appear in.
    async fn appears_in(&self) -> &[Episode] {
        &self.0.appears_in
    }

    /// The primary function of the droid.
    async fn primary_function(&self) -> &Option<&str> {
        &self.0.primary_function
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn hero<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(
            desc = "If omitted, returns the hero of the whole saga. If provided, returns the hero of that particular episode."
        )]
        episode: Option<Episode>,
    ) -> Character<'a> {
        let star_wars = ctx.data_unchecked::<StarWars>();
        match episode {
            Some(episode_name) => {
                if episode_name == Episode::Empire {
                    Human(star_wars.chars.get(star_wars.luke).unwrap()).into()
                } else {
                    Droid(star_wars.chars.get(star_wars.artoo).unwrap()).into()
                }
            }
            None => Human(star_wars.chars.get(star_wars.luke).unwrap()).into(),
        }
    }

    async fn human<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "id of the human")] id: String,
    ) -> Option<Human<'a>> {
        ctx.data_unchecked::<StarWars>().human(&id).map(Human)
    }

    async fn humans<'a>(
        &self,
        ctx: &Context<'a>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Connection<usize, Human<'a>>> {
        let humans = ctx.data_unchecked::<StarWars>().humans().to_vec();
        query_characters(after, before, first, last, &humans, Human).await
    }

    async fn droid<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "id of the droid")] id: String,
    ) -> Option<Droid<'a>> {
        ctx.data_unchecked::<StarWars>().droid(&id).map(Droid)
    }

    async fn droids<'a>(
        &self,
        ctx: &Context<'a>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Connection<usize, Droid<'a>>> {
        let droids = ctx.data_unchecked::<StarWars>().droids().to_vec();
        query_characters(after, before, first, last, &droids, Droid).await
    }
}

#[derive(Interface)]
#[graphql(
    field(name = "id", type = "&str"),
    field(name = "name", type = "&str"),
    field(name = "friends", type = "Vec<Character<'ctx>>"),
    field(name = "appears_in", type = "&[Episode]")
)]
pub enum Character<'a> {
    Human(Human<'a>),
    Droid(Droid<'a>),
}

async fn query_characters<'a, F, T>(
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
    characters: &[&'a StarWarsChar],
    map_to: F,
) -> Result<Connection<usize, T>>
where
    F: Fn(&'a StarWarsChar) -> T,
    T: OutputType,
{
    query(
        after,
        before,
        first,
        last,
        |after, before, first, last| async move {
            let mut start = 0usize;
            let mut end = characters.len();

            if let Some(after) = after {
                if after >= characters.len() {
                    return Ok(Connection::new(false, false));
                }
                start = after + 1;
            }

            if let Some(before) = before {
                if before == 0 {
                    return Ok(Connection::new(false, false));
                }
                end = before;
            }

            let mut slice = &characters[start..end];

            if let Some(first) = first {
                slice = &slice[..first.min(slice.len())];
                end -= first.min(slice.len());
            } else if let Some(last) = last {
                slice = &slice[slice.len() - last.min(slice.len())..];
                start = end - last.min(slice.len());
            }

            let mut connection = Connection::new(start > 0, end < characters.len());
            connection.edges.extend(
                slice
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| Edge::new(start + idx, (map_to)(item))),
            );
            Ok::<_, Error>(connection)
        },
    )
    .await
}
