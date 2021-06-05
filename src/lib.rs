use anyhow::{anyhow, Result};
use thiserror::Error;

pub enum DepKind {
    Normal,
    Development,
    Build,
}

pub enum LatestVersion {
    V(String),
    Removed,
}

pub struct OutdatedResult {
    projects: Vec<(String, Vec<OutdatedDep>)>,
}

pub struct OutdatedDep {
    name: String,
    project_version: String,
    latest_compatible_version: Option<String>,
    latest_version: LatestVersion,
    kind: DepKind,
    platform: Option<String>,
}

#[derive(Default)]
pub struct OutdatedDepBuilder {
    name: Option<String>,
    project_version: Option<String>,
    latest_compatible_version: Option<String>,
    latest_version: Option<LatestVersion>,
    kind: Option<DepKind>,
    platform: Option<String>,
}

#[derive(Debug)]
enum ParseState {
    Start,
    InWorkspaceTitle,
    InWorkspaceBreak,
    InHeader,
    InRow,
    InHeaderBreak,
}

impl OutdatedResult {
    pub fn try_from(output: &str) -> Result<Self> {
        let lines = output.split("\n");
        let mut deps = vec![];
        let mut state: Result<ParseState> = Ok(ParseState::Start);
        let mut cur_outdated_dep = OutdatedDepBuilder::default();
        let mut workspace_name: Option<String> = None;

        for line in lines.collect::<Vec<&str>>() {
            println!("current line\n{}", line);
            println!("Current state\n{:#?}", &state);
            state = match state? {
                ParseState::Start | ParseState::InWorkspaceTitle => {
                    // cur_outdated_dep.name = Some(line.to_owned());
                    workspace_name = Some(line.to_owned());
                    Ok(ParseState::InWorkspaceBreak)
                }
                ParseState::InWorkspaceBreak => {
                    if line != "================" {
                        Err(anyhow!("Expected line break, found [{}]", line))
                    } else {
                        Ok(ParseState::InHeader)
                    }
                }
                ParseState::InHeader => {
                    let parts = line.split_whitespace().collect::<Vec<&str>>();
                    if parts != vec!["Name", "Project", "Compat", "Latest", "Kind", "Platform"] {
                        Err(anyhow!("Expected header row, got [{}]", line))
                    } else {
                        Ok(ParseState::InHeaderBreak)
                    }
                }
                ParseState::InHeaderBreak => Ok(ParseState::InRow),
                ParseState::InRow => {
                    if line == "" {
                        Ok(ParseState::Start)
                    } else {
                        // This assumes that none of the rows have values which
                        // themselves have white space
                        let values = line.split_whitespace().collect::<Vec<&str>>();
                        if values.len() != 6 {
                            Err(anyhow!(
                                "Expected row [{}] to have 6 values, but it has [{}]",
                                &line,
                                line.len()
                            ))
                        } else {
                            cur_outdated_dep.name = Some(values[0].to_owned());
                            cur_outdated_dep.project_version = Some(values[1].to_owned());

                            cur_outdated_dep.latest_compatible_version = match values[2] {
                                "---" => None,
                                _ => Some(values[2].to_owned()),
                            };

                            cur_outdated_dep.latest_version = Some(match values[3] {
                                "Removed" => LatestVersion::Removed,
                                _ => LatestVersion::V(values[3].to_owned()),
                            });

                            let dep_kind = match values[4] {
                                "Normal" => Ok(DepKind::Normal),
                                "Development" => Ok(DepKind::Development),
                                "Build" => Ok(DepKind::Build),
                                _ => Err(anyhow!(
                                    "Unknown depdendency kind [{}]",
                                    values[4].to_owned()
                                )),
                            }?;
                            cur_outdated_dep.kind = Some(dep_kind);
                            cur_outdated_dep.platform = match values[5] {
                                "---" => None,
                                _ => Some(line.to_owned()),
                            };
                            Ok(ParseState::InRow)
                        }
                    }
                }
            };
            println!("State now \n{:#?}\n\n", &state);
        }
        // if cur_outdated_dep.kind.is_none(){
        //     Err(anyhow!("Parsed data is missing a 'kind'"))
        // }
        // if cur_outdated_dep.name.is_none(){
        //     Err(anyhow!("Parsed data is missing a 'name'"))
        // }
        // if cur_outdated_dep.project_version.is_none(){
        //     Err(anyhow!("Parsed data is missing a 'project version'"))
        // }
        // if cur_outdated_dep.latest_version.is_none(){
        //     Err(anyhow!("Parsed data is missing a 'latest version'"))
        // }

        deps.push((
            workspace_name
                .ok_or(anyhow!(
                    "Parsed data is missing a value for 'workspace name'"
                ))?
                .to_owned(),
            vec![OutdatedDep {
                name: cur_outdated_dep
                    .name
                    .ok_or(anyhow!("Parsed data is missing a value for 'name'"))?,
                kind: cur_outdated_dep
                    .kind
                    .ok_or(anyhow!("Parsed data is missing a value for 'kind'"))?,
                latest_version: cur_outdated_dep.latest_version.ok_or(anyhow!(
                    "Parsed data is missing a value for 'latest version'"
                ))?,
                project_version: cur_outdated_dep.project_version.ok_or(anyhow!(
                    "Parsed data is missing a value for 'project version'"
                ))?,
                platform: cur_outdated_dep.platform,
                latest_compatible_version: cur_outdated_dep.latest_compatible_version,
            }],
        ));
        Ok(OutdatedResult { projects: deps })
    }
}

#[cfg(test)]
mod tests {
    use crate::OutdatedResult;

    #[test]
    fn test_parse() {
        let output = r#"project-checker
================
Name         Project  Compat  Latest         Kind    Platform
----         -------  ------  ------         ----    --------
git2         0.9.2    ---     0.13.20        Normal  ---
libgit2-sys  0.8.2    ---     0.12.21+1.1.0  Normal  ---

scanner
================
Name         Project  Compat  Latest         Kind    Platform
----         -------  ------  ------         ----    --------
git2         0.9.2    ---     0.13.20        Normal  ---
libgit2-sys  0.8.2    ---     0.12.21+1.1.0  Normal  ---

foo
================
Name         Project  Compat  Latest         Kind    Platform
----         -------  ------  ------         ----    --------
clap             2.20.0   2.20.5  2.26.0   Normal       ---
clap->bitflags   0.7.0    ---     0.9.1    Normal       ---
clap->libc       0.2.18   0.2.29  Removed  Normal       ---
clap->term_size  0.2.1    0.2.3   0.3.0    Normal       ---
clap->vec_map    0.6.0    ---     0.8.0    Normal       ---
num_cpus         1.6.0    ---     1.6.2    Development  ---
num_cpus->libc   0.2.18   0.2.29  0.2.29   Normal       ---
pkg-config       0.3.8    0.3.9   0.3.9    Build        ---
term             0.4.5    ---     0.4.6    Normal       ---
term_size->libc  0.2.18   0.2.29  0.2.29   Normal       cfg(not(target_os = "windows")) 
        "#;
        let o = OutdatedResult::try_from(output).unwrap();
    }
}
