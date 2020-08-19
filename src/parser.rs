use anyhow::{bail, Error};
use pest::{iterators::Pair, Parser};

fn strip_folds(s: &str) -> String {
    let re = regex::Regex::new(r"\r?\n[\t ]").unwrap();

    re.replace_all(s, "").into_owned()
}

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct CalParser;

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub sub_components: Vec<Component>,
    pub properties: Vec<Property>,
}

impl Component {
    pub fn from_str_to_stream(data: &str) -> Result<Vec<Component>, Error> {
        let pairs = CalParser::parse(Rule::component, &data)?;

        pairs.map(Component::from_pair).collect()
    }

    fn from_pair(pair: Pair<Rule>) -> Result<Component, Error> {
        let span = pair.as_span();
        let mut name = None;
        let mut sub_components = Vec::new();
        let mut properties = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::name => name = Some(strip_folds(inner_pair.as_str())),
                Rule::component => sub_components.push(Component::from_pair(inner_pair)?),
                Rule::property => properties.push(Property::from_pair(inner_pair)?),
                _ => bail!("Unexpected type {:?}", inner_pair.as_rule()),
            }
        }

        if let Some(name) = name {
            Ok(Component {
                name,
                sub_components,
                properties,
            })
        } else {
            bail!("No name for component: {:?}", span.as_str());
        }
    }

    pub fn as_string(&self) -> String {
        let lines = self
            .properties
            .iter()
            .map(|v| v.as_string())
            .chain(self.sub_components.iter().map(|v| v.as_string()))
            .collect::<Vec<_>>()
            .join("\n");

        format!("BEGIN:{}\n{}\nEND:{}", self.name, lines, self.name)
    }
}

#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub parameters: Vec<Parameter>,
}

impl Property {
    fn from_pair(pair: Pair<Rule>) -> Result<Property, Error> {
        let span = pair.as_span();
        let mut name = None;
        let mut value = None;
        let mut parameters = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::name => name = Some(strip_folds(inner_pair.as_str())),
                Rule::property_value => value = Some(strip_folds(inner_pair.as_str())),
                Rule::param => parameters.push(Parameter::from_pair(inner_pair)?),
                _ => bail!("Unexpected type {:?}", inner_pair.as_rule()),
            }
        }

        if let (Some(name), Some(value)) = (name, value) {
            Ok(Property {
                name,
                value,
                parameters,
            })
        } else {
            bail!("No name for property: {:?}", span.as_str());
        }
    }

    pub fn as_string(&self) -> String {
        if self.parameters.is_empty() {
            format!("{}:{}", self.name, self.value)
        } else {
            let params = self
                .parameters
                .iter()
                .map(|v| v.as_string())
                .collect::<Vec<_>>()
                .join(";");

            format!("{};{}:{}", self.name, params, self.value)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub values: Vec<String>,
}

impl Parameter {
    fn from_pair(pair: Pair<Rule>) -> Result<Parameter, Error> {
        let span = pair.as_span();
        let mut name = None;
        let mut values = Vec::new();
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::name => name = Some(inner_pair.as_str().to_string()),
                Rule::param_value => values.push(inner_pair.as_str().trim_matches('"').to_string()),
                _ => bail!("Unexpected type {:?}", inner_pair.as_rule()),
            }
        }

        if values.is_empty() {
            bail!("No values for param {:?}", span.as_str());
        }

        if let Some(name) = name {
            Ok(Parameter { name, values })
        } else {
            bail!("No name for parameter: {:?}", span.as_str());
        }
    }

    pub fn as_string(&self) -> String {
        // We need to convert the values into a comma seperated string, quoting
        // values that need quoting.
        let values = self
            .values
            .iter()
            .map(|v| {
                if v.is_empty() || v.contains(&['`', ':', ';'] as &[_]) {
                    format!(r#""{}""#, v)
                } else {
                    v.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(",");

        format!("{}={}", self.name, values)
    }
}
