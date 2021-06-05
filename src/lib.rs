use anyhow::{anyhow, Result};

#[derive(Clone, Debug)]
pub enum DepKind {
    Normal,
    Development,
    Build,
}

#[derive(Clone, Debug)]
pub enum LatestVersion {
    V(String),
    Removed,
}

#[derive(Clone, Debug)]
pub struct OutdatedResult {
    projects: Vec<(String, Vec<OutdatedDep>)>,
}

#[derive(Clone, Debug)]
pub struct OutdatedDep {
    pub name: String,
    pub project_version: String,
    pub latest_compatible_version: Option<String>,
    pub latest_version: LatestVersion,
    pub kind: DepKind,
    pub platform: Option<String>,
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
    InWorkspaceBreak,
    InHeader,
    InRow,
    InHeaderBreak,
}

impl OutdatedResult {
    pub fn get_workspaces(&self) -> &Vec<(String, Vec<OutdatedDep>)> {
        &self.projects
    }

    pub fn try_from(output: &str) -> Result<Self> {
        let lines = output
            .split("\n")
            .into_iter()
            .map(|s| s.trim())
            .collect::<Vec<&str>>();
        let mut cur_deps: Vec<OutdatedDep> = vec![];
        let mut workspace_deps: Vec<(String, Vec<OutdatedDep>)> = vec![];
        let mut state: Result<ParseState> = Ok(ParseState::Start);
        let mut cur_outdated_dep = OutdatedDepBuilder::default();
        let mut workspace_name: Option<String> = None;
        let mut column_indices: Vec<usize> = vec![];

        for line in lines {
            state = match state? {
                ParseState::Start => {
                    // cur_outdated_dep.name = Some(line.to_owned());
                    if workspace_name.is_some() {
                        workspace_deps.push((workspace_name.clone().unwrap(), cur_deps.clone()));
                    }
                    column_indices = vec![];
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
                    let columns = vec!["Name", "Project", "Compat", "Latest", "Kind", "Platform"];
                    let parts = line.split_whitespace().collect::<Vec<&str>>();
                    if parts != columns {
                        Err(anyhow!("Expected header row, got [{}]", line))
                    } else {
                        // Extract the column indices so we can fetch the values
                        // once we're looking at the rows
                        for column in columns {
                            let i: (usize, &str) = line
                                .match_indices(column)
                                .into_iter()
                                .nth(0)
                                .ok_or(anyhow!(
                                "Expected to get an index for `{}` but none was found in line [{}]",
                                column,
                                line
                            ))?;
                            column_indices.push(i.0);
                        }
                        Ok(ParseState::InHeaderBreak)
                    }
                }
                ParseState::InHeaderBreak => Ok(ParseState::InRow),
                ParseState::InRow => {
                    if line == "" {
                        Ok(ParseState::Start)
                    } else {
                        for (i, start) in column_indices.iter().enumerate() {
                            // Fetch the value for the current column in the current row
                            let value = if i == 5 {
                                // If the current index is the last one, just
                                // grab the rest of the text
                                line[start.clone()..].trim()
                            } else {
                                // Otherwise, grab the text up to the next column
                                line[start.clone()..column_indices[i + 1]].trim()
                            };

                            match i {
                                0 => {
                                    cur_outdated_dep.name = Some(value.to_owned());
                                }
                                1 => {
                                    cur_outdated_dep.project_version = Some(value.to_owned());
                                }
                                2 => {
                                    cur_outdated_dep.latest_compatible_version = match value {
                                        "---" => None,
                                        _ => Some(value.to_owned()),
                                    };
                                }
                                3 => {
                                    cur_outdated_dep.latest_version = Some(match value {
                                        "Removed" => LatestVersion::Removed,
                                        _ => LatestVersion::V(value.to_owned()),
                                    });
                                }
                                4 => {
                                    let dep_kind = match value {
                                        "Normal" => Ok(DepKind::Normal),
                                        "Development" => Ok(DepKind::Development),
                                        "Build" => Ok(DepKind::Build),
                                        _ => Err(anyhow!(
                                            "Unknown dependency kind [{}]",
                                            value.to_owned()
                                        )),
                                    }?;
                                    cur_outdated_dep.kind = Some(dep_kind);
                                }
                                5 => {
                                    cur_outdated_dep.platform = match value {
                                        "---" => None,
                                        _ => Some(line.to_owned()),
                                    };
                                }
                                _ => {}
                            }
                        }
                        cur_deps.push(OutdatedDep {
                            name: cur_outdated_dep
                                .name
                                .clone()
                                .ok_or(anyhow!("Parsed data is missing a value for 'name'"))?,
                            kind: cur_outdated_dep
                                .kind
                                .clone()
                                .ok_or(anyhow!("Parsed data is missing a value for 'kind'"))?,
                            latest_version: cur_outdated_dep.latest_version.clone().ok_or(
                                anyhow!("Parsed data is missing a value for 'latest version'"),
                            )?,
                            project_version: cur_outdated_dep.project_version.clone().ok_or(
                                anyhow!("Parsed data is missing a value for 'project version'"),
                            )?,
                            platform: cur_outdated_dep.platform.clone(),
                            latest_compatible_version: cur_outdated_dep
                                .latest_compatible_version
                                .clone(),
                        });

                        Ok(ParseState::InRow)
                    }
                }
            }
        }

        workspace_deps.push((workspace_name.clone().unwrap(), cur_deps.clone()));
        Ok(OutdatedResult {
            projects: workspace_deps,
        })
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
Name             Project  Compat  Latest   Kind         Platform
----             -------  ------  ------   ----         --------
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
        OutdatedResult::try_from(output).unwrap();
    }
}
