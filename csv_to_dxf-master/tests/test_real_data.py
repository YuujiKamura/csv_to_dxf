import unittest
import pandas as pd
import numpy as np
import sys
import os

sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from gui.core.csv_processor import CSVProcessor

class TestWidthAssignE2E(unittest.TestCase):
    def test_section1_width_assign_left(self):
        csv_path = os.path.join('data', '区間1.csv')
        processor = CSVProcessor()
        self.assertTrue(processor.read_csv_file(csv_path))
        # 左幅員に割り当て
        df = processor.process_section_data('区間1', width_assign='wl')
        self.assertIsNotNone(df)
        self.assertIn('wl', df.columns)
        self.assertIn('wr', df.columns)
        # wlに値が入り、wrは0
        self.assertTrue((df['wr'] == 0).all())
        self.assertTrue((df['wl'] > 0).any())

    def test_section1_width_assign_right(self):
        csv_path = os.path.join('data', '区間1.csv')
        processor = CSVProcessor()
        self.assertTrue(processor.read_csv_file(csv_path))
        # 右幅員に割り当て
        df = processor.process_section_data('区間1', width_assign='wr')
        self.assertIsNotNone(df)
        self.assertIn('wl', df.columns)
        self.assertIn('wr', df.columns)
        # wrに値が入り、wlは0
        self.assertTrue((df['wl'] == 0).all())
        self.assertTrue((df['wr'] > 0).any())

    def test_section1_width_reassign(self):
        csv_path = os.path.join('data', '区間1.csv')
        processor = CSVProcessor()
        self.assertTrue(processor.read_csv_file(csv_path))
        # まず左に割り当て
        df_left = processor.process_section_data('区間1', width_assign='wl')
        self.assertTrue((df_left['wr'] == 0).all())
        self.assertTrue((df_left['wl'] > 0).any())
        # 次に右に再割り当て
        df_right = processor.process_section_data('区間1', width_assign='wr')
        self.assertTrue((df_right['wl'] == 0).all())
        self.assertTrue((df_right['wr'] > 0).any()) 