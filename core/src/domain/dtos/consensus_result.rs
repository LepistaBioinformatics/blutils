use super::{linnaean_ranks::LinnaeanRank, taxonomy_bean::TaxonomyBean};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryWithConsensus {
    pub run_id: Option<Uuid>,
    pub query: String,
    pub taxon: Option<TaxonomyBean>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryWithoutConsensus {
    pub query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConsensusResult {
    /// No consensus option
    ///
    /// This option should be used when the consensus checking process not found
    /// an appropriate taxonomy.
    NoConsensusFound(QueryWithoutConsensus),

    /// Consensus option
    ///
    /// This option should be used when the consensus checking process found an
    /// appropriate taxonomy.
    ConsensusFound(QueryWithConsensus),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsensusBean {
    pub rank: LinnaeanRank,
    pub identifier: String,
    pub occurrences: i32,
    pub taxonomy: Option<String>,
    pub accessions: Vec<String>,
}

impl ConsensusBean {
    pub(crate) fn from_taxonomy_bean(
        bean: TaxonomyBean,
        accession: Option<String>,
        taxonomy: String,
    ) -> Self {
        ConsensusBean {
            rank: bean.reached_rank,
            identifier: bean.identifier,
            occurrences: 0,
            taxonomy: taxonomy.into(),
            accessions: match accession {
                Some(res) => vec![res],
                _ => vec![],
            },
        }
    }

    pub(crate) fn fold_consensus_list(consensus: Vec<Self>) -> Vec<Self> {
        consensus
            .iter()
            .fold(HashMap::<String, Self>::new(), |mut acc, bean| {
                let rank = bean.rank.to_string();
                let identifier = bean.identifier.to_string();

                let consensus_bean = acc
                    .entry(format!("{}__{}", rank, identifier))
                    .or_insert(Self {
                        occurrences: 0,
                        accessions: vec![],
                        ..bean.clone()
                    });
                //
                // BUG-R13: the seed starts without accessions and every
                // accession is kept once, in first-seen order.
                //
                for accession in &bean.accessions {
                    if !consensus_bean.accessions.contains(accession) {
                        consensus_bean.accessions.push(accession.to_owned());
                    }
                }

                consensus_bean.occurrences += 1;

                acc
            })
            .into_iter()
            .map(|(_, bean)| bean)
            .collect::<Vec<Self>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taxonomy_bean(rank: LinnaeanRank, identifier: &str) -> TaxonomyBean {
        TaxonomyBean {
            reached_rank: rank,
            max_allowed_rank: None,
            identifier: identifier.to_string(),
            perc_identity: 99.0,
            bit_score: 100.0,
            taxonomy: None,
            mutated: false,
            single_match: false,
            consensus_beans: None,
        }
    }

    fn consensus_bean(
        rank: LinnaeanRank,
        identifier: &str,
        accessions: &[&str],
        taxonomy: &str,
    ) -> ConsensusBean {
        ConsensusBean {
            rank,
            identifier: identifier.to_string(),
            occurrences: 0,
            taxonomy: Some(taxonomy.to_string()),
            accessions: accessions.iter().map(|a| a.to_string()).collect(),
        }
    }

    fn sorted(mut beans: Vec<ConsensusBean>) -> Vec<ConsensusBean> {
        beans.sort_by_key(|b| (b.rank.to_string(), b.identifier.clone()));
        beans
    }

    #[test]
    fn from_taxonomy_bean_with_accession_sets_single_accession() {
        let bean = ConsensusBean::from_taxonomy_bean(
            taxonomy_bean(LinnaeanRank::Genus, "561"),
            Some("NR_1.1".to_string()),
            "d__2;g__561".to_string(),
        );

        assert_eq!(
            bean,
            ConsensusBean {
                rank: LinnaeanRank::Genus,
                identifier: "561".to_string(),
                occurrences: 0,
                taxonomy: Some("d__2;g__561".to_string()),
                accessions: vec!["NR_1.1".to_string()],
            }
        );
    }

    #[test]
    fn from_taxonomy_bean_without_accession_sets_empty_accessions() {
        let bean = ConsensusBean::from_taxonomy_bean(
            taxonomy_bean(LinnaeanRank::Species, "9"),
            None,
            "s__9".to_string(),
        );

        assert!(bean.accessions.is_empty());
        assert_eq!(bean.occurrences, 0);
        assert_eq!(bean.taxonomy, Some("s__9".to_string()));
    }

    #[test]
    fn fold_consensus_list_empty_returns_empty() {
        assert!(ConsensusBean::fold_consensus_list(vec![]).is_empty());
    }

    #[test]
    fn fold_consensus_list_same_rank_and_identifier_counts_occurrences() {
        let folded = ConsensusBean::fold_consensus_list(vec![
            consensus_bean(LinnaeanRank::Genus, "x", &["A"], "t1"),
            consensus_bean(LinnaeanRank::Genus, "x", &["B"], "t2"),
            consensus_bean(LinnaeanRank::Genus, "x", &["C"], "t3"),
        ]);

        assert_eq!(folded.len(), 1);
        assert_eq!(folded[0].occurrences, 3);
        assert_eq!(folded[0].accessions, vec!["A", "B", "C"]);
    }

    #[test]
    fn fold_consensus_list_first_taxonomy_wins() {
        let folded = ConsensusBean::fold_consensus_list(vec![
            consensus_bean(LinnaeanRank::Genus, "x", &["A"], "first"),
            consensus_bean(LinnaeanRank::Genus, "x", &["B"], "second"),
        ]);

        assert_eq!(folded[0].taxonomy, Some("first".to_string()));
    }

    #[test]
    fn fold_consensus_list_distinct_keys_kept_separate() {
        let folded = sorted(ConsensusBean::fold_consensus_list(vec![
            consensus_bean(LinnaeanRank::Genus, "x", &["A"], "t"),
            consensus_bean(LinnaeanRank::Species, "x", &["B"], "t"),
            consensus_bean(LinnaeanRank::Genus, "y", &["C"], "t"),
            consensus_bean(LinnaeanRank::Genus, "x", &["D"], "t"),
        ]));

        assert_eq!(
            folded
                .iter()
                .map(|b| (
                    format!("{}__{}", b.rank, b.identifier),
                    b.occurrences
                ))
                .collect::<Vec<_>>(),
            vec![
                ("g__x".to_string(), 2),
                ("g__y".to_string(), 1),
                ("s__x".to_string(), 1),
            ]
        );
    }

    // BUG-R13: non-adjacent repeated accessions are deduplicated.
    #[test]
    fn fold_consensus_list_non_adjacent_repeated_accession_deduplicated() {
        let folded = ConsensusBean::fold_consensus_list(vec![
            consensus_bean(LinnaeanRank::Genus, "x", &["A"], "t"),
            consensus_bean(LinnaeanRank::Genus, "x", &["B"], "t"),
            consensus_bean(LinnaeanRank::Genus, "x", &["A"], "t"),
        ]);

        assert_eq!(folded[0].accessions, vec!["A", "B"]);
    }

    // BUG-R13: the seed bean's accessions are not appended twice.
    #[test]
    fn fold_consensus_list_multi_accession_first_bean_kept_once() {
        let folded = ConsensusBean::fold_consensus_list(vec![consensus_bean(
            LinnaeanRank::Genus,
            "x",
            &["A", "B"],
            "t",
        )]);

        assert_eq!(folded[0].accessions, vec!["A", "B"]);
        assert_eq!(folded[0].occurrences, 1);
    }
}
