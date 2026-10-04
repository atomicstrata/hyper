import unittest
from datasets import synthetic
from scientific import canonical, construct, check

class ScientificAdapters(unittest.TestCase):
    def test_duplicate_incidence_is_rejected_before_library_normalization(self):
        graph = synthetic('invalid', 3, 1, 2, fixed=True)
        graph['hyperedges'][0]['vertices'].append(graph['hyperedges'][0]['vertices'][0])
        with self.assertRaisesRegex(ValueError, 'Duplicate incidence'):
            canonical(graph)

    def test_hnx_nested_properties_use_unflattened_public_dataframe(self):
        graph = synthetic('metadata', 3, 2, 3, fixed=True)
        result = check('hypernetx', construct('hypernetx', graph), graph)
        self.assertTrue(result['exact_canonical_roundtrip'])
        self.assertEqual(result['wrong_metadata'], [])

    def test_hgx_parallel_edges_are_reported_without_checker_failure(self):
        graph = synthetic('parallel', 3, 2, 2, fixed=True)
        graph['hyperedges'][1]['vertices'] = graph['hyperedges'][0]['vertices'][:]
        result = check('hypergraphx', construct('hypergraphx', graph), graph)
        self.assertTrue(result['opaque_record_values_preserved'])
        self.assertFalse(result['native_multiplicity_preserved'])
        self.assertFalse(result['exact_canonical_roundtrip'])
        self.assertEqual(result['native_hyperedge_count'], 1)

if __name__ == '__main__': unittest.main()
