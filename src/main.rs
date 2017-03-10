#[macro_use(itry)]
extern crate iron;
extern crate router;
extern crate logger;
extern crate env_logger;
extern crate urlencoded;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate askama;
extern crate rusqlite;

use iron::prelude::*;
use iron::headers::ContentType;
use iron::Chain;
use iron::status;
use router::Router;
use askama::Template;
use logger::Logger;

mod errors {
    error_chain! { }
}

use errors::Error;

mod model;

#[derive(Template)]
#[template(path = "user_log.html")]
struct UserLogTemplate {
    username: String,
    games: Vec<String>,
}

#[derive(Template)]
#[template(path = "signup_form.html")]
struct SignupFormTemplate {}

fn home(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "Welcome!")))
}

fn user_log(req: &mut Request) -> IronResult<Response> {
    let params = itry!(
        req.extensions.get::<Router>().ok_or::<Error>(
            "no router".into()
        )
    );

    let user_string = itry!(
        params.find("user").ok_or::<Error>(
            "no user id or username provided".into()
        )
    );

    let user = itry!(model::get_user_from_id_or_name(user_string.to_string()), (status::NotFound, "User not found"));

    let template_context = UserLogTemplate {
        username: user.username,
        games: itry!(model::get_user_game_names(user.id)),
    };

    let mut response = Response::with((
        status::Ok,
        template_context.render(),
    ));

    response.headers.set(ContentType::html());

    Ok(response)
}

fn signup_form(_: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        SignupFormTemplate{}.render(),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn main() {
    env_logger::init().unwrap();

    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/log/:user", user_log, "user_log");
    router.get("/signup", signup_form, "signup_form");

    let mut chain = Chain::new(router);

    let (logger_before, logger_after) = Logger::new(None);
    chain.link_before(logger_before);
    chain.link_after(logger_after);

    Iron::new(chain).http("0.0.0.0:3000").unwrap();
}
