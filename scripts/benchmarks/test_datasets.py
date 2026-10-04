import unittest
from datasets import synthetic, incidence, hif, changed

class Datasets(unittest.TestCase):
    def test_seed_and_membership_counts_are_stable(self):
        a=synthetic('moderate',1000,2000,8)
        self.assertEqual(a,synthetic('moderate',1000,2000,8))
        self.assertEqual(len(a['vertices']),1000)
        self.assertEqual(len(a['hyperedges']),2000)
        self.assertTrue(all(len(e['vertices'])==len(set(e['vertices'])) for e in a['hyperedges']))
    def test_incidence_keeps_empty_edges_and_isolates(self):
        graph=synthetic('correctness',30,15,8)
        graph['hyperedges'][0]['vertices']=[]
        scene=incidence(graph)
        self.assertEqual(len(scene['nodes']),45)
        self.assertEqual(len(scene['links']),sum(len(e['vertices']) for e in graph['hyperedges']))
        self.assertEqual(len(hif(graph)['nodes']),30)
        self.assertEqual(hif(graph)['edges'][0]['edge'],'g0')
    def test_updates_change_one_percent_without_renaming_vertices(self):
        graph=synthetic('moderate',1000,2000,8)
        updated=changed(graph)
        self.assertEqual([v['id'] for v in graph['vertices']], [v['id'] for v in updated['vertices']])
        self.assertEqual(sum(a!=b for a,b in zip(graph['hyperedges'],updated['hyperedges'])),20)
if __name__=='__main__': unittest.main()
