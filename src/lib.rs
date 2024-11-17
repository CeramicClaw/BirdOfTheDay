use std::{env, fs::File, io::{Read, Write}};

use rand::Rng;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const LOCAL_BIRDS: &str = "birds.json";

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
struct Bird {
    #[serde(rename(deserialize = "sciName"))]
    pub scientific_name: String,
    #[serde(rename(deserialize = "comName"))]
    pub common_name: String,
    #[serde(rename(deserialize = "speciesCode"))]
    pub species_code: String,
    #[serde(rename(deserialize = "category"))]
    pub category: String,
    #[serde(rename(deserialize = "taxonOrder"))]
    pub taxon_order: f32,
    #[serde(rename(deserialize = "bandingCodes"))]
    pub banding_codes: Option<Vec<String>>,
    #[serde(rename(deserialize = "comNameCodes"))]
    pub com_name_codes: Option<Vec<String>>,
    #[serde(rename(deserialize = "sciNameCodes"))]
    pub sci_name_codes: Option<Vec<String>>,
    #[serde(rename(deserialize = "order"))]
    pub order: Option<String>,
    #[serde(rename(deserialize = "familyComName"))]
    pub family_com_name: Option<String>,
    #[serde(rename(deserialize = "familySciName"))]
    pub family_sci_name: Option<String>,
    #[serde(rename(deserialize = "reportAs"))]
    pub report_as: Option<String>,
    #[serde(rename(deserialize = "extinct"))]
    pub extinct: Option<bool>,
    #[serde(rename(deserialize = "extinctYear"))]
    pub extinct_year: Option<i32>,
    #[serde(rename(deserialize = "familyCode"))]
    pub family_code: Option<String>,
}

struct BirdImage {
    photo_type: String,
    url_download: String,
    url_source: String,
    alt_text: String,
}

#[derive(Debug)]
struct Token {
    token: String,
    did: String,
}

pub fn run() -> bool {
    let b = match get_bird() {
        Some(b) => b,
        None => return false,
    };

    let image = match get_bird_photo(&b) {
        Some(id) => id,
        None => return false,
    };

    let token = match authenticate() {
        Some(t) => t,
        None => return false,
    };

    return post(&b, &image, &token);
}

/// Download a copy of *all* birds and save a copy to the local machine
/// This should only be run periodically
pub fn get_all_birds() {
    // Get all available birds from eBird.org
    let r = match minreq::get("https://api.ebird.org/v2/ref/taxonomy/ebird?fmt=json")
        .with_header("X-eBirdApiToken", env::var("EBIRD_API_KEY").unwrap())
        .with_timeout(30)
        .send() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error reading response from eBird call: {}", e);
                return;
            }
    };

    if r.status_code != 200 {
        eprintln!("Bad response code from eBird: {}", r.status_code);
        return;
    }

    let mut file = match File::create(LOCAL_BIRDS) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error creating 'birds.json': {}", e);
            return;
        }
    };

    if let Err(e) = file.write_all(r.as_bytes()) {
        eprintln!("Error writing data to 'birds.json': {}", e);
    }
}

/// Get one random bird from eBird.org
fn get_bird() -> Option<Bird> {
    // Read in the local copy of all data from eBird.org
    let mut file = match File::open(LOCAL_BIRDS) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening 'birds.json': {}", e);
            return None;
        }
    };

    let mut contents = String::new();
    if let Err(e) = file.read_to_string(&mut contents) {
        eprintln!("Error opening 'birds.json': {}", e);
        return None;
    }

    let mut birds: Vec<Bird> = match serde_json::from_str(&contents) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error converting eBird response into JSON: {}", e);
            return None;
        }
    };

    // Filter out all birds that are species and are extinct
    birds.retain(|b| !b.common_name.contains("sp.") && b.extinct.is_none());
    
    // Finally, get a random bird
    let mut rng = rand::thread_rng();
    Some(birds[rng.gen_range(0..birds.len())].clone())
}

/// Get a photo of the desired bird
fn get_bird_photo(bird: &Bird) -> Option<BirdImage> {
    let r = match minreq::get(format!("https://ebird.org/species/{}", bird.species_code))
        .with_header("User-Agent", format!("BirdOfTheDayBot ({})", env::var("BOTD_EMAIL").unwrap()))
        .with_timeout(30)
        .send() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error reading bird image response: {}", e);
                return None;
            }
    };

    if r.status_code != 200 {
        eprintln!("Bad response code eBird while getting image: {}", r.status_code);
        return None;
    }

    let page = match r.as_str() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error converting eBird page into string: {}", e);
            return None;
        }
    };
    // Now extract all the image properties
    let doc = Html::parse_document(page);   
    let s_url_download = Selector::parse(r#"meta[property="og:image"]"#).unwrap();
    let url_download: &str = match doc.select(&s_url_download).next() {
        Some(s) => s.value().attr("content").unwrap(),
        None => {
            eprintln!("No 'og:image' tag found in html: {}", doc.html());
            return None;
        }
    };
    let s_alt_text = Selector::parse(r#"meta[property="og:image:alt"]"#).unwrap();
    let alt_text: &str = match doc.select(&s_alt_text).next() {
        Some(s) => s.value().attr("content").unwrap(),
        None => {
            eprintln!("No 'og:image:alt' tag found in html: {}", doc.html());
            return None;
        }
    };
    let s_url_source = Selector::parse(r#"meta[property="og:url"]"#).unwrap();
    let url_source: &str = match doc.select(&s_url_source).next() {
        Some(s) => s.value().attr("content").unwrap(),
        None => {
            eprintln!("No 'og:url' tag found in html: {}", doc.html());
            return None;
        }
    };
    let s_url_source = Selector::parse(r#"link[rel="image_src"]"#).unwrap();
    let photo_type: &str = match doc.select(&s_url_source).next() {
        Some(s) => s.value().attr("type").unwrap(),
        None => {
            eprintln!("No 'image_src' tag found in html: {}", doc.html());
            return None;
        }
    };

    return Some(BirdImage {
        photo_type: photo_type.to_string(),
        url_download: url_download.to_string(),
        url_source: url_source.to_string(),
        alt_text: alt_text.to_string()
    });
}

/// Authenticate username/password and get the `accessJwt` and `did` values
fn authenticate() -> Option<Token> {
    let json = json!({
        "identifier": format!("{}", env::var("BOTD_HANDLE").unwrap()),
        "password": format!("{}", env::var("BOTD_PASS").unwrap()),
    });
    let r = match minreq::post("https://bsky.social/xrpc/com.atproto.server.createSession")
        .with_header("Content-Type", "application/json")
        .with_body(json.to_string())
        .with_timeout(30)
        .send() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error during session authentication: {}", e);
                return None;
            }
    };

    if r.status_code != 200 {
        eprintln!("Error during authentication: {}", r.as_str().unwrap());
        return None;
    }
    
    let json = match r.json::<Value>() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Successfully recieved token, but error occurred during conversion to JSON: {}", e);
            return None;
        }
    };
    
    let token = match json.get("accessJwt") {
        Some(t) => t.as_str().unwrap(),
        None => {
            eprintln!("Successfully converted response to JSON, but 'accessJwt' parameter was not present");
            return None;
        }
    };

    let did = match json.get("did") {
        Some(t) => t.as_str().unwrap(),
        None => {
            eprintln!("Successfully converted response to JSON, but 'did' parameter was not present");
            return None;
        }
    };

    return Some(Token{ token: token.to_string(), did: did.to_string()});
}

/// Make a Bluesky post
fn post(b: &Bird, photo: &BirdImage, token: &Token) -> bool {
    // Get and upload the image card
    let r_photo = match minreq::get(photo.url_download.clone())
        .with_header("User-Agent", format!("BirdOfTheDayBot ({})", env::var("BOTD_EMAIL").unwrap()))
        .with_timeout(30)
        .send() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error reading Macaulay Library response: {}", e);
                return false;
            }
        };

    if r_photo.status_code != 200 {
        eprintln!("Error during photo download (URL: {}): {}", photo.url_download, r_photo.as_str().unwrap());
        return false;
    }

    let blob = match minreq::post("https://bsky.social/xrpc/com.atproto.repo.uploadBlob")
        .with_header("Content-Type", photo.photo_type.clone())
        .with_header("Authorization", format!("Bearer {}", token.token))
        .with_body(r_photo.as_bytes())
        .with_timeout(30)
        .send() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error during photo upload: {}", e);
                return false;
            }
        };
    
    if blob.status_code != 200 {
        eprintln!("Error from photo upload (Response code {})", blob.status_code);
        return false;
    }

    let blob_json = match blob.json::<Value>() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error converting photo upload to JSON: {}", e);
            return false;
        }
    };
    
    // Image card upload was successful, now make the post
    let text = format!("{} ({})\n\nImage Credit", b.common_name, b.scientific_name);
    let post_json = json!({
        "repo": token.did,
        "collection": "app.bsky.feed.post",
        "record": {
            "$type": "app.bsky.feed.post",
            "text": text,
            "facets": [
                {
                "index": {
                    "byteStart": text.len() - "Image Credit".len(),
                    "byteEnd": text.len(),
                },
                "features": [{
                    "$type": "app.bsky.richtext.facet#link",
                    "uri": photo.url_source
                }]
                }
            ],
            "createdAt": OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
            "embed": {
                "$type": "app.bsky.embed.images",
                "images": [{
                        "alt": photo.alt_text,
                        "image": blob_json.get("blob").unwrap(),
                    }],  
                }
            }
        });
    
    let post = match minreq::post("https://bsky.social/xrpc/com.atproto.repo.createRecord")
        .with_header("Content-Type", "application/json")
        .with_header("Authorization", format!("Bearer {}", token.token))
        .with_body(post_json.to_string())
        .with_timeout(30)
        .send() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error during post creation: {}", e);
                return false;
            }
        };
    
    if post.status_code != 200 {
        eprintln!("Post creation unsuccessful: {}", post.as_str().unwrap());
        return false;
    }

    println!("Success!!!!");
    return true;
}
