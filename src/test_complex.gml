graph [
  label "Test gml file"
  directed 0
  # this is a comment
  node [
    # this is another comment
    id 0
    label "test0"
    a 1.1
    b 2.5555555
  ]
  node [
    id 1
    label "test1"
    a 5
    c 0.0
  ]
  node [
    id 2
    label "test2"
    a 367.0618067245002
    b 0.03230269781364574
    c 0.04307026375152765
  ]
  node [
    id 3
    label "test3"
  ]
  node [
    id 4
    label "test4"
    dict_1 [
      item 1.0
   ]
   nodeitems [
       item 0.5
       item 1
       item 2
       item 3
   ]
  ]
  edge [
     source 1
     target 2
     value 1.1000
     dict_2 [
       item1 1.0
       item2 1.0
     ]
  ]
  edge [
     source 3
     target 4
     list [
        item 0.5
        item 1
        item 2
        item 3
    ]
  ]
]
