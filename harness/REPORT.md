# Differential test report

vulture 2.16 on CPython 3.14 versus condor, run by `harness/difftest.py` on the
corpus in `harness/corpus.toml`. condor deliberately differs from vulture
(spec/improvements.md): "only vulture" counts findings condor no longer
reports, "only condor" counts new ones. Exit codes are compared.

Runs: 79. Runs with identical findings: 19. Runs with different exit codes: 0.

| Project | Kind | Revision | Flags | vulture | condor | Only vulture | Only condor | Exit | vulture (s) | condor (s) |
|---|---|---|---|---:|---:|---:|---:|---|---:|---:|
| vulture | library (the reference itself) | b0f67ba00446 | default | 0 | 0 | 0 | 0 | 0 | 0.617 | 0.024 |
| vulture | library (the reference itself) | b0f67ba00446 | min_confidence | 0 | 0 | 0 | 0 | 0 | 0.615 | 0.023 |
| vulture | library (the reference itself) | b0f67ba00446 | sort_by_size | 0 | 0 | 0 | 0 | 0 | 0.617 | 0.024 |
| vulture | library (the reference itself) | b0f67ba00446 | make_whitelist | 0 | 0 | 0 | 0 | 0 | 0.614 | 0.023 |
| vulture | library (the reference itself) | b0f67ba00446 | ignores | 0 | 0 | 0 | 0 | 0 | 0.639 | 0.021 |
| vulture | library (the reference itself) | b0f67ba00446 | exclude | 0 | 0 | 0 | 0 | 0 | 0.6 | 0.018 |
| requests | library | 0e322af87745 | default | 93 | 93 | 0 | 0 | 3 | 0.721 | 0.027 |
| requests | library | 0e322af87745 | min_confidence | 11 | 11 | 0 | 0 | 3 | 0.712 | 0.027 |
| requests | library | 0e322af87745 | sort_by_size | 93 | 93 | 0 | 0 | 3 | 0.659 | 0.028 |
| requests | library | 0e322af87745 | make_whitelist | 93 | 93 | 0 | 0 | 3 | 0.677 | 0.025 |
| requests | library | 0e322af87745 | ignores | 86 | 86 | 0 | 0 | 3 | 0.68 | 0.025 |
| requests | library | 0e322af87745 | exclude | 59 | 59 | 0 | 0 | 3 | 0.604 | 0.013 |
| flask | library / web framework | c12a5d874c5a | default | 333 | 80 | 253 | 0 | 3 | 0.729 | 0.04 |
| flask | library / web framework | c12a5d874c5a | min_confidence | 35 | 17 | 18 | 0 | 3 | 0.684 | 0.038 |
| flask | library / web framework | c12a5d874c5a | sort_by_size | 333 | 80 | 253 | 0 | 3 | 0.705 | 0.034 |
| flask | library / web framework | c12a5d874c5a | make_whitelist | 333 | 80 | 253 | 0 | 3 | 0.71 | 0.031 |
| flask | library / web framework | c12a5d874c5a | ignores | 150 | 77 | 73 | 0 | 3 | 0.667 | 0.035 |
| flask | library / web framework | c12a5d874c5a | exclude | 60 | 51 | 9 | 0 | 3 | 0.548 | 0.019 |
| click | library | 934813e4d421 | default | 119 | 58 | 61 | 0 | 3 | 0.684 | 0.03 |
| click | library | 934813e4d421 | sort_by_size | 119 | 58 | 61 | 0 | 3 | 0.7 | 0.029 |
| click | library | 934813e4d421 | make_whitelist | 119 | 58 | 61 | 0 | 3 | 0.714 | 0.03 |
| click | library | 934813e4d421 | exclude | 70 | 38 | 32 | 0 | 3 | 0.622 | 0.018 |
| attrs | library, uses __all__ | 598494a61841 | default | 119 | 81 | 38 | 0 | 3 | 0.692 | 0.038 |
| attrs | library, uses __all__ | 598494a61841 | min_confidence | 1 | 1 | 0 | 0 | 3 | 0.683 | 0.035 |
| attrs | library, uses __all__ | 598494a61841 | make_whitelist | 119 | 81 | 38 | 0 | 3 | 0.704 | 0.038 |
| attrs | library, uses __all__ | 598494a61841 | exclude | 28 | 15 | 13 | 0 | 3 | 0.544 | 0.014 |
| httpx | library, uses __all__ | 26d48e0634e6 | default | 63 | 7 | 56 | 0 | 3 | 0.728 | 0.037 |
| httpx | library, uses __all__ | 26d48e0634e6 | min_confidence | 4 | 4 | 0 | 0 | 3 | 0.723 | 0.034 |
| httpx | library, uses __all__ | 26d48e0634e6 | sort_by_size | 63 | 7 | 56 | 0 | 3 | 0.719 | 0.034 |
| httpx | library, uses __all__ | 26d48e0634e6 | ignores | 62 | 7 | 55 | 0 | 3 | 0.712 | 0.033 |
| rich | library | 43d3b04725ab | default | 130 | 111 | 19 | 0 | 3 | 0.983 | 0.055 |
| rich | library | 43d3b04725ab | sort_by_size | 130 | 111 | 19 | 0 | 3 | 0.96 | 0.051 |
| rich | library | 43d3b04725ab | make_whitelist | 130 | 111 | 19 | 0 | 3 | 0.966 | 0.05 |
| rich | library | 43d3b04725ab | exclude | 132 | 122 | 10 | 0 | 3 | 0.832 | 0.028 |
| black | application; test data contains unusual syntax | 1b2427a2b785 | default | 1793 | 1772 | 25 | 4 | 3 | 1.811 | 0.059 |
| black | application; test data contains unusual syntax | 1b2427a2b785 | min_confidence | 374 | 356 | 18 | 0 | 3 | 1.669 | 0.055 |
| black | application; test data contains unusual syntax | 1b2427a2b785 | ignores | 1738 | 1718 | 24 | 4 | 3 | 1.768 | 0.052 |
| black | application; test data contains unusual syntax | 1b2427a2b785 | exclude | 123 | 121 | 3 | 1 | 3 | 1.42 | 0.042 |
| fastapi | web framework + many FastAPI apps in docs_src | bb8c2a64981d | default | 1664 | 174 | 1490 | 0 | 3 | 1.911 | 0.144 |
| fastapi | web framework + many FastAPI apps in docs_src | bb8c2a64981d | min_confidence | 28 | 28 | 0 | 0 | 3 | 1.662 | 0.174 |
| fastapi | web framework + many FastAPI apps in docs_src | bb8c2a64981d | ignores | 570 | 146 | 424 | 0 | 3 | 1.73 | 0.161 |
| fastapi | web framework + many FastAPI apps in docs_src | bb8c2a64981d | exclude | 125 | 34 | 91 | 0 | 3 | 0.708 | 0.028 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | default | 50 | 26 | 24 | 0 | 3 | 0.559 | 0.017 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | min_confidence | 1 | 1 | 0 | 0 | 3 | 0.584 | 0.017 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | sort_by_size | 50 | 26 | 24 | 0 | 3 | 0.553 | 0.016 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | make_whitelist | 50 | 26 | 24 | 0 | 3 | 0.537 | 0.016 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | ignores | 46 | 26 | 20 | 0 | 3 | 0.534 | 0.016 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | exclude | 60 | 27 | 33 | 0 | 3 | 0.533 | 0.014 |
| pydantic | library | 5bd3a6507b74 | default | 1362 | 509 | 853 | 0 | 3 | 2.264 | 0.647 |
| pydantic | library | 5bd3a6507b74 | min_confidence | 49 | 19 | 30 | 0 | 3 | 2.133 | 0.681 |
| pydantic | library | 5bd3a6507b74 | exclude | 219 | 180 | 39 | 0 | 3 | 1.141 | 0.039 |
| scrapy | application framework | b1f9e56693cd | default | 438 | 423 | 17 | 2 | 3 | 1.8 | 0.141 |
| scrapy | application framework | b1f9e56693cd | sort_by_size | 438 | 423 | 17 | 2 | 3 | 1.694 | 0.113 |
| scrapy | application framework | b1f9e56693cd | exclude | 371 | 364 | 7 | 0 | 3 | 1.01 | 0.03 |
| celery | application framework | 92514ac88afc | default | 875 | 799 | 76 | 0 | 3 | 1.964 | 0.186 |
| celery | application framework | 92514ac88afc | min_confidence | 57 | 25 | 32 | 0 | 3 | 1.848 | 0.178 |
| celery | application framework | 92514ac88afc | exclude | 843 | 768 | 75 | 0 | 3 | 1.829 | 0.176 |
| django | large web framework | 2d4add11fd57 | default | 3180 | 2006 | 1174 | 0 | 3 | 8.483 | 0.545 |
| django | large web framework | 2d4add11fd57 | min_confidence | 90 | 90 | 0 | 0 | 3 | 8.222 | 0.462 |
| django | large web framework | 2d4add11fd57 | exclude | 1979 | 1838 | 149 | 8 | 3 | 2.83 | 0.214 |
| netbox | Django application | b89601d93dae | default | 2912 | 2096 | 816 | 0 | 3 | 4.198 | 0.186 |
| netbox | Django application | b89601d93dae | min_confidence | 28 | 28 | 0 | 0 | 3 | 4.215 | 0.176 |
| netbox | Django application | b89601d93dae | ignores | 2807 | 1994 | 813 | 0 | 3 | 4.796 | 0.179 |
| netbox | Django application | b89601d93dae | exclude | 2463 | 1824 | 639 | 0 | 3 | 3.569 | 0.156 |
| saleor | large Django application | 1ce97784dcdc | default | 3263 | 1533 | 1730 | 0 | 3 | 9.021 | 0.847 |
| saleor | large Django application | 1ce97784dcdc | exclude | 1921 | 1655 | 266 | 0 | 3 | 3.123 | 0.189 |
| sqlalchemy | large library | 66e31814644b | default | 2528 | 2087 | 450 | 9 | 3 | 7.934 | 0.611 |
| sqlalchemy | large library | 66e31814644b | min_confidence | 463 | 56 | 407 | 0 | 3 | 7.829 | 0.649 |
| typeshed | stub files (.pyi passed explicitly; directories only yield .py) | 5a6569574a1d | default | 3709 | 1566 | 2146 | 3 | 3 | 0.932 | 0.019 |
| typeshed | stub files (.pyi passed explicitly; directories only yield .py) | 5a6569574a1d | min_confidence | 2143 | 1 | 2142 | 0 | 3 | 0.833 | 0.018 |
| typeshed | stub files (.pyi passed explicitly; directories only yield .py) | 5a6569574a1d | make_whitelist | 3709 | 1566 | 2146 | 3 | 3 | 0.928 | 0.019 |
| cpython-stdlib | CPython 3.14 standard library and its tests (bad syntax and encodings included) | CPython 3.14 | default | 5610 | 5196 | 421 | 7 | 3 | 16.879 | 0.457 |
| cpython-stdlib | CPython 3.14 standard library and its tests (bad syntax and encodings included) | CPython 3.14 | min_confidence | 344 | 342 | 2 | 0 | 3 | 16.059 | 0.407 |
| cpython-stdlib | CPython 3.14 standard library and its tests (bad syntax and encodings included) | CPython 3.14 | sort_by_size | 5610 | 5196 | 421 | 7 | 3 | 16.145 | 0.356 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | default | 1687 | 1686 | 2 | 1 | 3 | 0.94 | 0.031 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | min_confidence | 1126 | 1126 | 0 | 0 | 3 | 0.866 | 0.039 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | sort_by_size | 1687 | 1686 | 2 | 1 | 3 | 0.947 | 0.034 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | make_whitelist | 1687 | 1686 | 2 | 1 | 3 | 0.973 | 0.033 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | ignores | 1684 | 1684 | 1 | 1 | 3 | 1.01 | 0.039 |

## Samples of differences (default flags)

### flask

Only vulture (253, first 10):
```
examples/javascript/tests/conftest.py:6: unused function 'fixture_app' (60% confidence)
examples/tutorial/flaskr/auth.py:32: unused function 'load_logged_in_user' (60% confidence)
examples/tutorial/flaskr/blog.py:60: unused function 'create' (60% confidence)
src/flask/__init__.py:46: unused function '__getattr__' (60% confidence)
src/flask/config.py:30: unused variable 'owner' (100% confidence)
src/flask/config.py:33: unused variable 'owner' (100% confidence)
tests/conftest.py:139: unused function 'modules_tmp_path_prefix' (60% confidence)
tests/conftest.py:37: unused function '_reset_os_environ' (60% confidence)
tests/conftest.py:85: unused function 'leak_detector' (60% confidence)
tests/conftest.py:99: unused function 'limit_loader' (60% confidence)
```

### click

Only vulture (61, first 10):
```
examples/aliases/aliases.py:107: unused function 'pull' (60% confidence)
examples/aliases/aliases.py:113: unused function 'clone' (60% confidence)
examples/aliases/aliases.py:119: unused function 'commit' (60% confidence)
examples/aliases/aliases.py:125: unused function 'status' (60% confidence)
examples/completion/completion.py:12: unused function 'ls' (60% confidence)
examples/completion/completion.py:24: unused function 'show_env' (60% confidence)
examples/completion/completion.py:50: unused function 'select_user' (60% confidence)
examples/imagepipe/imagepipe.py:101: unused function 'save_cmd' (60% confidence)
examples/imagepipe/imagepipe.py:121: unused function 'display_cmd' (60% confidence)
examples/imagepipe/imagepipe.py:131: unused function 'resize_cmd' (60% confidence)
```

### attrs

Only vulture (38, first 10):
```
bench/test_benchmarks.py:12: unused variable 'pytestmark' (60% confidence)
tests/dataclass_transform_example.py:18: unused variable 'with_converter' (60% confidence)
tests/test_compat.py:12: unused function '_mp' (60% confidence)
tests/test_dunders.py:447: unused variable 'foo_value' (60% confidence)
tests/test_dunders.py:452: unused variable 'foo_value' (60% confidence)
tests/test_dunders.py:457: unused variable 'foo_value' (60% confidence)
tests/test_funcs.py:244: unused variable 'lat' (60% confidence)
tests/test_funcs.py:245: unused variable 'lon' (60% confidence)
tests/test_funcs.py:249: unused variable 'coords' (60% confidence)
tests/test_funcs.py:275: unused variable 'coords' (60% confidence)
```

### httpx

Only vulture (56, first 10):
```
httpx/_status_codes.py:100: unused variable 'PARTIAL_CONTENT' (60% confidence)
httpx/_status_codes.py:101: unused variable 'MULTI_STATUS' (60% confidence)
httpx/_status_codes.py:102: unused variable 'ALREADY_REPORTED' (60% confidence)
httpx/_status_codes.py:103: unused variable 'IM_USED' (60% confidence)
httpx/_status_codes.py:106: unused variable 'MULTIPLE_CHOICES' (60% confidence)
httpx/_status_codes.py:110: unused variable 'NOT_MODIFIED' (60% confidence)
httpx/_status_codes.py:111: unused variable 'USE_PROXY' (60% confidence)
httpx/_status_codes.py:116: unused variable 'BAD_REQUEST' (60% confidence)
httpx/_status_codes.py:117: unused variable 'UNAUTHORIZED' (60% confidence)
httpx/_status_codes.py:118: unused variable 'PAYMENT_REQUIRED' (60% confidence)
```

### rich

Only vulture (19, first 10):
```
examples/attrs.py:15: unused variable 'z' (60% confidence)
examples/attrs.py:20: unused variable 'point1' (60% confidence)
examples/attrs.py:21: unused variable 'point2' (60% confidence)
examples/attrs.py:22: unused variable 'point3' (60% confidence)
examples/attrs.py:28: unused variable 'triangles' (60% confidence)
rich/pretty.py:497: unused variable 'parent' (60% confidence)
rich/pretty.py:498: unused variable 'is_root' (60% confidence)
rich/pretty.py:967: unused variable 'price' (60% confidence)
rich/pretty.py:968: unused variable 'category' (60% confidence)
rich/pretty.py:969: unused variable 'reviews' (60% confidence)
```

### black

Only vulture (25, first 10):
```
scripts/diff_shades_gha_helper.py:161: unused function 'comment_body' (60% confidence)
scripts/diff_shades_gha_helper.py:198: unused function 'comment_details' (60% confidence)
scripts/release_tests.py:24: unused class 'TestRelease' (60% confidence)
src/black/schema.py:6: unused function 'get_schema' (60% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:137: unused variable 'buffer_input' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:138: unused variable 'func_input' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:139: unused variable 'float_arg' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:205: unused variable 'bbbbbbbbbbbbbbbbbb' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:212: unused variable 'bbbbbbbbbbbbbbbbbb' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:295: unused variable 'buffer_input' (100% confidence)
```
Only condor (4, first 10):
```
tests/data/cases/comments6.py:118: unused variable 'aaaaaaaaaaaaa' (60% confidence)
tests/data/cases/comments6.py:118: unused variable 'bbbbbbbbb' (60% confidence)
tests/data/cases/comments6.py:55: unused variable 'an_element_with_a_long_value' (60% confidence)
tests/test_black.py:3036: unused function 'tracefunc' (60% confidence)
```

### fastapi

Only vulture (1490, first 10):
```
.github/actions/notify-translations/app/main.py:181: unused variable 'github_event_name' (60% confidence)
docs_src/additional_responses/tutorial001.py:18: unused function 'read_item' (60% confidence)
docs_src/additional_responses/tutorial002.py:16: unused function 'read_item' (60% confidence)
docs_src/additional_responses/tutorial003.py:18: unused function 'read_item' (60% confidence)
docs_src/additional_responses/tutorial004.py:23: unused function 'read_item' (60% confidence)
docs_src/additional_status_codes/tutorial001.py:11: unused function 'upsert_item' (60% confidence)
docs_src/additional_status_codes/tutorial001_an.py:12: unused function 'upsert_item' (60% confidence)
docs_src/additional_status_codes/tutorial001_an_py310.py:11: unused function 'upsert_item' (60% confidence)
docs_src/additional_status_codes/tutorial001_an_py39.py:11: unused function 'upsert_item' (60% confidence)
docs_src/additional_status_codes/tutorial001_py310.py:9: unused function 'upsert_item' (60% confidence)
```

### full-stack-fastapi-template

Only vulture (24, first 10):
```
backend/app/api/routes/items.py:13: unused function 'read_items' (60% confidence)
backend/app/api/routes/items.py:44: unused function 'read_item' (60% confidence)
backend/app/api/routes/items.py:71: unused function 'update_item' (60% confidence)
backend/app/api/routes/items.py:95: unused function 'delete_item' (60% confidence)
backend/app/api/routes/login.py:101: unused function 'recover_password_html_content' (60% confidence)
backend/app/api/routes/login.py:24: unused function 'login_access_token' (60% confidence)
backend/app/api/routes/login.py:46: unused function 'test_token' (60% confidence)
backend/app/api/routes/login.py:54: unused function 'recover_password' (60% confidence)
backend/app/api/routes/login.py:78: unused function 'reset_password' (60% confidence)
backend/app/api/routes/users.py:100: unused function 'update_password_me' (60% confidence)
```

### pydantic

Only vulture (853, first 10):
```
.github/actions/people/people.py:260: unused variable 'labels' (60% confidence)
docs/plugins/conversion_table.py:29: unused variable 'valid_examples' (60% confidence)
docs/plugins/conversion_table.py:30: unused variable 'invalid_examples' (60% confidence)
docs/plugins/conversion_table.py:31: unused variable 'core_schemas' (60% confidence)
pydantic/_internal/_core_metadata.py:36: unused variable 'pydantic_js_prefer_positional_arguments' (60% confidence)
pydantic/_internal/_decorators_v1.py:116: unused variable '__values' (100% confidence)
pydantic/_internal/_decorators_v1.py:122: unused variable '__info' (100% confidence)
pydantic/_internal/_decorators_v1.py:122: unused variable '__values' (100% confidence)
pydantic/_internal/_decorators_v1.py:129: unused variable '__fields_tuple' (100% confidence)
pydantic/_internal/_decorators_v1.py:129: unused variable '__info' (100% confidence)
```

### scrapy

Only vulture (17, first 10):
```
scrapy/__init__.py:32: unused function '__getattr__' (60% confidence)
tests/test_engine.py:55: unused variable 'price' (60% confidence)
tests/test_engine.py:62: unused variable 'price' (60% confidence)
tests/test_item.py:290: unused class 'ItemMetaClassCellRegression' (60% confidence)
tests/test_pipeline_files.py:297: unused variable 'file_urls' (60% confidence)
tests/test_pipeline_files.py:300: unused variable 'custom_file_urls' (60% confidence)
tests/test_pipeline_files.py:314: unused variable 'file_urls' (60% confidence)
tests/test_pipeline_files.py:317: unused variable 'custom_file_urls' (60% confidence)
tests/test_pipeline_images.py:281: unused variable 'image_urls' (60% confidence)
tests/test_pipeline_images.py:284: unused variable 'custom_image_urls' (60% confidence)
```
Only condor (2, first 10):
```
tests/spiders.py:446: unused method 'parse_async' (60% confidence)
tests/spiders.py:461: unused method 'parse_async_gen' (60% confidence)
```

### celery

Only vulture (76, first 10):
```
celery/backends/base.py:676: unused variable 'chord_id' (100% confidence)
celery/bin/logtool.py:129: unused function 'traces' (60% confidence)
celery/bin/logtool.py:143: unused function 'incomplete' (60% confidence)
examples/django/demoapp/migrations/0001_initial.py:10: unused variable 'dependencies' (60% confidence)
examples/django/demoapp/migrations/0001_initial.py:13: unused variable 'operations' (60% confidence)
examples/django/demoapp/migrations/0001_initial.py:8: unused variable 'initial' (60% confidence)
examples/django/demoapp/tasks.py:23: unused function 'count_widgets' (60% confidence)
examples/django/demoapp/tasks.py:28: unused function 'rename_widget' (60% confidence)
examples/django/proj/celery.py:20: unused method 'debug_task' (60% confidence)
examples/eventlet/tasks.py:6: unused function 'urlopen' (60% confidence)
```

### django

Only vulture (1174, first 10):
```
django/contrib/admin/apps.py:20: unused class 'AdminConfig' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:350: unused function 'result_list_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:37: unused function 'paginator_number' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:465: unused function 'date_hierarchy_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:489: unused function 'search_form_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:500: unused function 'admin_list_filter' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:521: unused function 'admin_actions_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:528: unused function 'change_list_object_tools_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:74: unused function 'pagination_tag' (60% confidence)
django/contrib/admin/templatetags/admin_modify.py:115: unused function 'submit_row_tag' (60% confidence)
```

### netbox

Only vulture (816, first 10):
```
contrib/gunicorn.py:15: unused variable 'max_requests' (60% confidence)
contrib/gunicorn.py:16: unused variable 'max_requests_jitter' (60% confidence)
contrib/gunicorn.py:2: unused variable 'bind' (60% confidence)
contrib/gunicorn.py:9: unused variable 'threads' (60% confidence)
netbox/account/migrations/0001_initial.py:10: unused variable 'dependencies' (60% confidence)
netbox/account/migrations/0001_initial.py:14: unused variable 'operations' (60% confidence)
netbox/circuits/apps.py:4: unused class 'CircuitsConfig' (60% confidence)
netbox/circuits/apps.py:8: unused method 'ready' (60% confidence)
netbox/circuits/forms/bulk_edit.py:146: unused variable 'install_date' (60% confidence)
netbox/circuits/forms/bulk_edit.py:151: unused variable 'termination_date' (60% confidence)
```

### saleor

Only vulture (1730, first 10):
```
.semgrep/no-use-requests-lib.fixed.py:49: redundant if-condition (100% confidence)
.semgrep/no-use-requests-lib.py:49: redundant if-condition (100% confidence)
saleor/account/app.py:5: unused class 'AccountAppConfig' (60% confidence)
saleor/account/app.py:8: unused method 'ready' (60% confidence)
saleor/account/error_codes.py:17: unused variable 'INVALID_PASSWORD' (60% confidence)
saleor/account/error_codes.py:24: unused variable 'PASSWORD_ENTIRELY_NUMERIC' (60% confidence)
saleor/account/error_codes.py:25: unused variable 'PASSWORD_TOO_COMMON' (60% confidence)
saleor/account/error_codes.py:26: unused variable 'PASSWORD_TOO_SHORT' (60% confidence)
saleor/account/error_codes.py:27: unused variable 'PASSWORD_TOO_SIMILAR' (60% confidence)
saleor/account/error_codes.py:44: unused variable 'BLANK' (60% confidence)
```

### sqlalchemy

Only vulture (450, first 10):
```
lib/sqlalchemy/engine/base.py:242: unused variable 'logging_token' (100% confidence)
lib/sqlalchemy/engine/base.py:245: unused variable 'stream_results' (100% confidence)
lib/sqlalchemy/engine/base.py:3039: unused variable 'logging_token' (100% confidence)
lib/sqlalchemy/engine/create.py:53: unused variable 'convert_unicode' (100% confidence)
lib/sqlalchemy/engine/create.py:56: unused variable 'echo_pool' (100% confidence)
lib/sqlalchemy/engine/create.py:74: unused variable 'pool_logging_name' (100% confidence)
lib/sqlalchemy/engine/create.py:75: unused variable 'pool_pre_ping' (100% confidence)
lib/sqlalchemy/engine/create.py:77: unused variable 'pool_recycle' (100% confidence)
lib/sqlalchemy/engine/create.py:78: unused variable 'pool_reset_on_return' (100% confidence)
lib/sqlalchemy/engine/create.py:79: unused variable 'pool_timeout' (100% confidence)
```
Only condor (9, first 10):
```
lib/sqlalchemy/dialects/mysql/base.py:1499: unused method 'visit_typeclause' (60% confidence)
lib/sqlalchemy/orm/bulk_persistence.py:1152: unused class 'BulkORMInsert' (60% confidence)
lib/sqlalchemy/orm/strategies.py:1464: unused class 'SubqueryLoader' (60% confidence)
lib/sqlalchemy/orm/strategies.py:376: unused class 'DeferredColumnLoader' (60% confidence)
lib/sqlalchemy/sql/compiler.py:2642: unused method 'visit_typeclause' (60% confidence)
lib/sqlalchemy/sql/compiler.py:3989: unused method 'visit_cte' (60% confidence)
lib/sqlalchemy/sql/naming.py:191: unused function '_constraint_name' (60% confidence)
lib/sqlalchemy/testing/util.py:83: unused class 'RandomSet' (60% confidence)
test/typing/plain_files/orm/relationship.py:89: unused class 'SelfReferential' (60% confidence)
```

### typeshed

Only vulture (2146, first 10):
```
stdlib/ast.pyi:1029: unused variable 'comparators' (100% confidence)
stdlib/ast.pyi:1029: unused variable 'left' (100% confidence)
stdlib/ast.pyi:1029: unused variable 'ops' (100% confidence)
stdlib/ast.pyi:1032: unused variable 'comparators' (100% confidence)
stdlib/ast.pyi:1032: unused variable 'left' (100% confidence)
stdlib/ast.pyi:1032: unused variable 'ops' (100% confidence)
stdlib/ast.pyi:1036: unused variable 'comparators' (100% confidence)
stdlib/ast.pyi:1036: unused variable 'left' (100% confidence)
stdlib/ast.pyi:1036: unused variable 'ops' (100% confidence)
stdlib/ast.pyi:1046: unused variable 'func' (100% confidence)
```
Only condor (3, first 10):
```
stdlib/builtins.pyi:1557: unused class 'range' (60% confidence)
stdlib/builtins.pyi:2242: unused class 'sentinel' (60% confidence)
stdlib/builtins.pyi:2287: unused class 'zip' (60% confidence)
```

### cpython-stdlib

Only vulture (421, first 10):
```
_colorize.py:178: unused variable 'keyword_constant' (60% confidence)
_colorize.py:185: unused variable 'soft_keyword' (60% confidence)
_pyrepl/completing_reader.py:240: unused variable 'cmpltn_message_visible' (60% confidence)
_pyrepl/console.py:74: unused variable 'xy' (100% confidence)
compression/zstd/__init__.py:231: unused variable 'dfast' (60% confidence)
compression/zstd/__init__.py:232: unused variable 'greedy' (60% confidence)
compression/zstd/__init__.py:234: unused variable 'lazy2' (60% confidence)
compression/zstd/__init__.py:235: unused variable 'btlazy2' (60% confidence)
compression/zstd/__init__.py:236: unused variable 'btopt' (60% confidence)
compression/zstd/__init__.py:237: unused variable 'btultra' (60% confidence)
```
Only condor (7, first 10):
```
_pylong.py:528: unused function 'int_divmod' (60% confidence)
test/test_compile.py:1647: unused function 'name_4' (60% confidence)
test/test_tarfile.py:2168: unused variable 'test_create_taropen' (60% confidence)
test/test_tarfile.py:2169: unused variable 'test_create_existing_taropen' (60% confidence)
test/test_tarfile.py:2537: unused variable 'test_unicode_filename_error' (60% confidence)
test/test_tarfile.py:2573: unused variable 'test_append_compressed' (60% confidence)
test/test_tarfile.py:896: unused variable 'test_fail_comp' (60% confidence)
```

### edge-cases

Only vulture (2, first 10):
```
dunder/module.py:1: unused function '__getattr__' (60% confidence)
usage/all_and_strings.py:22: unused function 'g' (60% confidence)
```
Only condor (1, first 10):
```
decorators/decorators.py:6: unused class 'Widget' (60% confidence)
```

