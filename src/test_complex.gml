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
    dict_str [
        a value_1
        b value_2
    ]
  ]
  node [
    id 1
    label "test1"
    a 5
    c 0.0
    list_str [
        a test_1
        a test_2
    ]
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
    list_1 [
       a 0.1
       a 0.2
    ]
  ]
  node [
    id 4
    label "test4"
    dict_1 [
      item_float 1.0
    ]
    list_1 [
       a 0.2
       a 0.3
    ]
  ]
  edge [
     source 1
     target 2
     value_double 1.1000
     dict_2 [
       item_1 1
       item_2 2
     ]
  ]
  edge [
     source 3
     target 4
     value_int 3
     list_2 [
        b 1
        b 2
        b 3
        b 4
    ]
  ]
]
