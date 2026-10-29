# Differential test report

vulture 2.16 on CPython 3.14 versus condor, run by `harness/difftest.py` on the
corpus in `harness/corpus.toml`. condor deliberately differs from vulture
(docs/improvements.md): "only vulture" counts findings condor no longer
reports, "only condor" counts new ones. Exit codes are compared.

Runs: 79. Runs with identical findings: 19. Runs with different exit codes: 0.

| Project | Kind | Revision | Flags | vulture | condor | Only vulture | Only condor | Exit | vulture (s) | condor (s) |
|---|---|---|---|---:|---:|---:|---:|---|---:|---:|
| vulture | library (the reference itself) | b0f67ba00446 | default | 0 | 0 | 0 | 0 | 0 | 0.25 | 0.304 |
| vulture | library (the reference itself) | b0f67ba00446 | min_confidence | 0 | 0 | 0 | 0 | 0 | 0.25 | 0.301 |
| vulture | library (the reference itself) | b0f67ba00446 | sort_by_size | 0 | 0 | 0 | 0 | 0 | 0.321 | 0.228 |
| vulture | library (the reference itself) | b0f67ba00446 | make_whitelist | 0 | 0 | 0 | 0 | 0 | 0.249 | 0.3 |
| vulture | library (the reference itself) | b0f67ba00446 | ignores | 0 | 0 | 0 | 0 | 0 | 0.194 | 0.025 |
| vulture | library (the reference itself) | b0f67ba00446 | exclude | 0 | 0 | 0 | 0 | 0 | 0.146 | 0.017 |
| requests | library | 0e322af87745 | default | 93 | 93 | 0 | 0 | 3 | 0.307 | 0.038 |
| requests | library | 0e322af87745 | min_confidence | 11 | 11 | 0 | 0 | 3 | 0.294 | 0.04 |
| requests | library | 0e322af87745 | sort_by_size | 93 | 93 | 0 | 0 | 3 | 0.331 | 0.039 |
| requests | library | 0e322af87745 | make_whitelist | 93 | 93 | 0 | 0 | 3 | 0.324 | 0.04 |
| requests | library | 0e322af87745 | ignores | 86 | 86 | 0 | 0 | 3 | 0.34 | 0.038 |
| requests | library | 0e322af87745 | exclude | 59 | 59 | 0 | 0 | 3 | 0.225 | 0.02 |
| flask | library / web framework | c12a5d874c5a | default | 333 | 81 | 252 | 0 | 3 | 0.515 | 0.079 |
| flask | library / web framework | c12a5d874c5a | min_confidence | 35 | 17 | 18 | 0 | 3 | 0.473 | 0.067 |
| flask | library / web framework | c12a5d874c5a | sort_by_size | 333 | 81 | 252 | 0 | 3 | 0.497 | 0.057 |
| flask | library / web framework | c12a5d874c5a | make_whitelist | 333 | 81 | 252 | 0 | 3 | 0.555 | 0.063 |
| flask | library / web framework | c12a5d874c5a | ignores | 150 | 77 | 73 | 0 | 3 | 0.563 | 0.066 |
| flask | library / web framework | c12a5d874c5a | exclude | 60 | 52 | 8 | 0 | 3 | 0.33 | 0.032 |
| click | library | 934813e4d421 | default | 119 | 58 | 61 | 0 | 3 | 0.609 | 0.05 |
| click | library | 934813e4d421 | sort_by_size | 119 | 58 | 61 | 0 | 3 | 0.618 | 0.053 |
| click | library | 934813e4d421 | make_whitelist | 119 | 58 | 61 | 0 | 3 | 0.633 | 0.051 |
| click | library | 934813e4d421 | exclude | 70 | 38 | 32 | 0 | 3 | 0.446 | 0.026 |
| attrs | library, uses __all__ | 598494a61841 | default | 119 | 78 | 41 | 0 | 3 | 0.577 | 0.072 |
| attrs | library, uses __all__ | 598494a61841 | min_confidence | 1 | 1 | 0 | 0 | 3 | 0.573 | 0.062 |
| attrs | library, uses __all__ | 598494a61841 | make_whitelist | 119 | 78 | 41 | 0 | 3 | 0.576 | 0.091 |
| attrs | library, uses __all__ | 598494a61841 | exclude | 28 | 16 | 12 | 0 | 3 | 0.285 | 0.024 |
| httpx | library, uses __all__ | 26d48e0634e6 | default | 63 | 7 | 56 | 0 | 3 | 0.688 | 0.079 |
| httpx | library, uses __all__ | 26d48e0634e6 | min_confidence | 4 | 4 | 0 | 0 | 3 | 0.685 | 0.069 |
| httpx | library, uses __all__ | 26d48e0634e6 | sort_by_size | 63 | 7 | 56 | 0 | 3 | 0.734 | 0.063 |
| httpx | library, uses __all__ | 26d48e0634e6 | ignores | 62 | 7 | 55 | 0 | 3 | 0.724 | 0.065 |
| rich | library | 43d3b04725ab | default | 130 | 113 | 17 | 0 | 3 | 1.139 | 0.094 |
| rich | library | 43d3b04725ab | sort_by_size | 130 | 113 | 17 | 0 | 3 | 1.134 | 0.105 |
| rich | library | 43d3b04725ab | make_whitelist | 130 | 113 | 17 | 0 | 3 | 1.128 | 0.091 |
| rich | library | 43d3b04725ab | exclude | 132 | 122 | 10 | 0 | 3 | 0.818 | 0.052 |
| black | application; test data contains unusual syntax | 1b2427a2b785 | default | 1793 | 1775 | 22 | 4 | 3 | 2.595 | 0.111 |
| black | application; test data contains unusual syntax | 1b2427a2b785 | min_confidence | 374 | 356 | 18 | 0 | 3 | 2.325 | 0.093 |
| black | application; test data contains unusual syntax | 1b2427a2b785 | ignores | 1738 | 1721 | 21 | 4 | 3 | 2.587 | 0.092 |
| black | application; test data contains unusual syntax | 1b2427a2b785 | exclude | 123 | 121 | 3 | 1 | 3 | 1.824 | 0.059 |
| fastapi | web framework + many FastAPI apps in docs_src | bb8c2a64981d | default | 1664 | 177 | 1487 | 0 | 3 | 2.739 | 0.301 |
| fastapi | web framework + many FastAPI apps in docs_src | bb8c2a64981d | min_confidence | 28 | 28 | 0 | 0 | 3 | 2.5 | 0.283 |
| fastapi | web framework + many FastAPI apps in docs_src | bb8c2a64981d | ignores | 570 | 148 | 422 | 0 | 3 | 2.551 | 0.297 |
| fastapi | web framework + many FastAPI apps in docs_src | bb8c2a64981d | exclude | 125 | 34 | 91 | 0 | 3 | 0.574 | 0.108 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | default | 50 | 26 | 24 | 0 | 3 | 0.322 | 0.033 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | min_confidence | 1 | 1 | 0 | 0 | 3 | 0.263 | 0.029 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | sort_by_size | 50 | 26 | 24 | 0 | 3 | 0.266 | 0.03 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | make_whitelist | 50 | 26 | 24 | 0 | 3 | 0.271 | 0.029 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | ignores | 46 | 26 | 20 | 0 | 3 | 0.29 | 0.025 |
| full-stack-fastapi-template | FastAPI application | 7e5ca88dc262 | exclude | 60 | 27 | 33 | 0 | 3 | 0.278 | 0.027 |
| pydantic | library | 5bd3a6507b74 | default | 1362 | 514 | 848 | 0 | 3 | 2.715 | 0.211 |
| pydantic | library | 5bd3a6507b74 | min_confidence | 49 | 19 | 30 | 0 | 3 | 2.576 | 0.216 |
| pydantic | library | 5bd3a6507b74 | exclude | 219 | 181 | 38 | 0 | 3 | 1.147 | 0.058 |
| scrapy | application framework | b1f9e56693cd | default | 438 | 422 | 18 | 2 | 3 | 1.798 | 0.162 |
| scrapy | application framework | b1f9e56693cd | sort_by_size | 438 | 422 | 18 | 2 | 3 | 1.724 | 0.161 |
| scrapy | application framework | b1f9e56693cd | exclude | 371 | 365 | 6 | 0 | 3 | 0.771 | 0.042 |
| celery | application framework | 92514ac88afc | default | 875 | 798 | 77 | 0 | 3 | 2.297 | 0.216 |
| celery | application framework | 92514ac88afc | min_confidence | 57 | 24 | 33 | 0 | 3 | 2.139 | 0.265 |
| celery | application framework | 92514ac88afc | exclude | 843 | 767 | 76 | 0 | 3 | 2.168 | 0.223 |
| django | large web framework | 2d4add11fd57 | default | 3180 | 2146 | 1036 | 2 | 3 | 11.42 | 0.763 |
| django | large web framework | 2d4add11fd57 | min_confidence | 90 | 90 | 0 | 0 | 3 | 10.883 | 0.836 |
| django | large web framework | 2d4add11fd57 | exclude | 1979 | 1865 | 122 | 8 | 3 | 3.415 | 0.564 |
| netbox | Django application | b89601d93dae | default | 2912 | 2116 | 796 | 0 | 3 | 5.215 | 0.24 |
| netbox | Django application | b89601d93dae | min_confidence | 28 | 28 | 0 | 0 | 3 | 5.063 | 0.23 |
| netbox | Django application | b89601d93dae | ignores | 2807 | 2011 | 796 | 0 | 3 | 5.831 | 0.254 |
| netbox | Django application | b89601d93dae | exclude | 2463 | 1844 | 619 | 0 | 3 | 4.615 | 0.212 |
| saleor | large Django application | 1ce97784dcdc | default | 3263 | 1478 | 1785 | 0 | 3 | 12.288 | 0.927 |
| saleor | large Django application | 1ce97784dcdc | exclude | 1921 | 1556 | 365 | 0 | 3 | 3.688 | 0.269 |
| sqlalchemy | large library | 66e31814644b | default | 2528 | 1968 | 569 | 9 | 3 | 10.393 | 0.979 |
| sqlalchemy | large library | 66e31814644b | min_confidence | 463 | 56 | 407 | 0 | 3 | 10.099 | 0.838 |
| typeshed | stub files (.pyi passed explicitly; directories only yield .py) | be1b2840d27e | default | 3709 | 1566 | 2146 | 3 | 3 | 0.751 | 0.022 |
| typeshed | stub files (.pyi passed explicitly; directories only yield .py) | be1b2840d27e | min_confidence | 2143 | 1 | 2142 | 0 | 3 | 0.599 | 0.02 |
| typeshed | stub files (.pyi passed explicitly; directories only yield .py) | be1b2840d27e | make_whitelist | 3709 | 1566 | 2146 | 3 | 3 | 0.728 | 0.023 |
| cpython-stdlib | CPython 3.14 standard library and its tests (bad syntax and encodings included) | CPython 3.14 | default | 5610 | 5266 | 353 | 9 | 3 | 21.632 | 0.642 |
| cpython-stdlib | CPython 3.14 standard library and its tests (bad syntax and encodings included) | CPython 3.14 | min_confidence | 344 | 328 | 16 | 0 | 3 | 20.191 | 0.646 |
| cpython-stdlib | CPython 3.14 standard library and its tests (bad syntax and encodings included) | CPython 3.14 | sort_by_size | 5610 | 5266 | 353 | 9 | 3 | 20.442 | 0.523 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | default | 1687 | 1687 | 1 | 1 | 3 | 0.623 | 0.04 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | min_confidence | 1126 | 1126 | 0 | 0 | 3 | 0.584 | 0.036 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | sort_by_size | 1687 | 1687 | 1 | 1 | 3 | 0.652 | 0.037 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | make_whitelist | 1687 | 1687 | 1 | 1 | 3 | 0.634 | 0.041 |
| edge-cases | generated: syntax features, noqa, type comments, encodings, errors | generated by | ignores | 1684 | 1684 | 1 | 1 | 3 | 0.621 | 0.042 |

## Samples of differences (default flags)

### flask

Only vulture (252, first 10):
```
examples/javascript/tests/conftest.py:6: unused function 'fixture_app' (60% confidence)
examples/tutorial/flaskr/auth.py:32: unused function 'load_logged_in_user' (60% confidence)
examples/tutorial/flaskr/blog.py:60: unused function 'create' (60% confidence)
src/flask/config.py:30: unused variable 'owner' (100% confidence)
src/flask/config.py:33: unused variable 'owner' (100% confidence)
tests/conftest.py:139: unused function 'modules_tmp_path_prefix' (60% confidence)
tests/conftest.py:37: unused function '_reset_os_environ' (60% confidence)
tests/conftest.py:85: unused function 'leak_detector' (60% confidence)
tests/conftest.py:99: unused function 'limit_loader' (60% confidence)
tests/test_appctx.py:117: unused function 'cleanup' (60% confidence)
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

Only vulture (41, first 10):
```
tests/dataclass_transform_example.py:18: unused variable 'with_converter' (60% confidence)
tests/test_annotations.py:108: unused variable 'cls_var' (60% confidence)
tests/test_annotations.py:406: unused variable 'cls_var1' (60% confidence)
tests/test_annotations.py:407: unused variable 'cls_var2' (60% confidence)
tests/test_annotations.py:408: unused variable 'cls_var3' (60% confidence)
tests/test_annotations.py:433: unused variable 'cls_var' (60% confidence)
tests/test_compat.py:12: unused function '_mp' (60% confidence)
tests/test_dunders.py:447: unused variable 'foo_value' (60% confidence)
tests/test_dunders.py:452: unused variable 'foo_value' (60% confidence)
tests/test_dunders.py:457: unused variable 'foo_value' (60% confidence)
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

Only vulture (17, first 10):
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

Only vulture (22, first 10):
```
scripts/diff_shades_gha_helper.py:161: unused function 'comment_body' (60% confidence)
scripts/diff_shades_gha_helper.py:198: unused function 'comment_details' (60% confidence)
src/black/schema.py:6: unused function 'get_schema' (60% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:137: unused variable 'buffer_input' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:138: unused variable 'func_input' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:139: unused variable 'float_arg' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:205: unused variable 'bbbbbbbbbbbbbbbbbb' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:212: unused variable 'bbbbbbbbbbbbbbbbbb' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:295: unused variable 'buffer_input' (100% confidence)
tests/data/cases/funcdef_return_type_trailing_comma.py:296: unused variable 'func_input' (100% confidence)
```
Only condor (4, first 10):
```
tests/data/cases/comments6.py:118: unused variable 'aaaaaaaaaaaaa' (60% confidence)
tests/data/cases/comments6.py:118: unused variable 'bbbbbbbbb' (60% confidence)
tests/data/cases/comments6.py:55: unused variable 'an_element_with_a_long_value' (60% confidence)
tests/test_black.py:3036: unused function 'tracefunc' (60% confidence)
```

### fastapi

Only vulture (1487, first 10):
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

Only vulture (848, first 10):
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

Only vulture (18, first 10):
```
tests/test_crawler.py:153: unreachable code after 'return' (100% confidence)
tests/test_crawler.py:234: unreachable code after 'return' (100% confidence)
tests/test_crawler.py:313: unreachable code after 'return' (100% confidence)
tests/test_crawler.py:392: unreachable code after 'return' (100% confidence)
tests/test_engine.py:55: unused variable 'price' (60% confidence)
tests/test_engine.py:62: unused variable 'price' (60% confidence)
tests/test_pipeline_files.py:297: unused variable 'file_urls' (60% confidence)
tests/test_pipeline_files.py:300: unused variable 'custom_file_urls' (60% confidence)
tests/test_pipeline_files.py:314: unused variable 'file_urls' (60% confidence)
tests/test_pipeline_files.py:317: unused variable 'custom_file_urls' (60% confidence)
```
Only condor (2, first 10):
```
tests/spiders.py:446: unused method 'parse_async' (60% confidence)
tests/spiders.py:461: unused method 'parse_async_gen' (60% confidence)
```

### celery

Only vulture (77, first 10):
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

Only vulture (1036, first 10):
```
django/contrib/admin/templatetags/admin_list.py:350: unused function 'result_list_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:37: unused function 'paginator_number' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:465: unused function 'date_hierarchy_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:489: unused function 'search_form_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:500: unused function 'admin_list_filter' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:521: unused function 'admin_actions_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:528: unused function 'change_list_object_tools_tag' (60% confidence)
django/contrib/admin/templatetags/admin_list.py:74: unused function 'pagination_tag' (60% confidence)
django/contrib/admin/templatetags/admin_modify.py:115: unused function 'submit_row_tag' (60% confidence)
django/contrib/admin/templatetags/admin_modify.py:122: unused function 'change_form_object_tools_tag' (60% confidence)
```
Only condor (2, first 10):
```
tests/servers/tests.py:189: unused class 'LiveServerViews' (60% confidence)
tests/servers/tests.py:317: unused class 'SingleThreadLiveServerViews' (60% confidence)
```

### netbox

Only vulture (796, first 10):
```
contrib/gunicorn.py:15: unused variable 'max_requests' (60% confidence)
contrib/gunicorn.py:16: unused variable 'max_requests_jitter' (60% confidence)
contrib/gunicorn.py:2: unused variable 'bind' (60% confidence)
contrib/gunicorn.py:9: unused variable 'threads' (60% confidence)
netbox/account/migrations/0001_initial.py:10: unused variable 'dependencies' (60% confidence)
netbox/account/migrations/0001_initial.py:14: unused variable 'operations' (60% confidence)
netbox/circuits/apps.py:8: unused method 'ready' (60% confidence)
netbox/circuits/forms/bulk_edit.py:146: unused variable 'install_date' (60% confidence)
netbox/circuits/forms/bulk_edit.py:151: unused variable 'termination_date' (60% confidence)
netbox/circuits/forms/bulk_edit.py:156: unused variable 'commit_rate' (60% confidence)
```

### saleor

Only vulture (1785, first 10):
```
.semgrep/no-use-requests-lib.fixed.py:49: redundant if-condition (100% confidence)
.semgrep/no-use-requests-lib.py:49: redundant if-condition (100% confidence)
saleor/account/app.py:8: unused method 'ready' (60% confidence)
saleor/account/error_codes.py:17: unused variable 'INVALID_PASSWORD' (60% confidence)
saleor/account/error_codes.py:24: unused variable 'PASSWORD_ENTIRELY_NUMERIC' (60% confidence)
saleor/account/error_codes.py:25: unused variable 'PASSWORD_TOO_COMMON' (60% confidence)
saleor/account/error_codes.py:26: unused variable 'PASSWORD_TOO_SHORT' (60% confidence)
saleor/account/error_codes.py:27: unused variable 'PASSWORD_TOO_SIMILAR' (60% confidence)
saleor/account/error_codes.py:44: unused variable 'BLANK' (60% confidence)
saleor/account/i18n_valid_address_extension.py:7: unused variable 'CITY_AREA' (60% confidence)
```

### sqlalchemy

Only vulture (569, first 10):
```
examples/sharding/asyncio.py:110: unused variable 'location_id' (60% confidence)
examples/sharding/asyncio.py:114: unused variable 'report_time' (60% confidence)
examples/sharding/separate_databases.py:94: unused variable 'location_id' (60% confidence)
examples/sharding/separate_databases.py:98: unused variable 'report_time' (60% confidence)
examples/sharding/separate_schema_translates.py:89: unused variable 'location_id' (60% confidence)
examples/sharding/separate_schema_translates.py:93: unused variable 'report_time' (60% confidence)
examples/sharding/separate_tables.py:107: unused variable 'location_id' (60% confidence)
examples/sharding/separate_tables.py:111: unused variable 'report_time' (60% confidence)
lib/sqlalchemy/engine/base.py:242: unused variable 'logging_token' (100% confidence)
lib/sqlalchemy/engine/base.py:245: unused variable 'stream_results' (100% confidence)
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

Only vulture (353, first 10):
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
Only condor (9, first 10):
```
_pylong.py:528: unused function 'int_divmod' (60% confidence)
test/test_compile.py:1647: unused function 'name_4' (60% confidence)
test/test_tarfile.py:1067: unused class 'GzipBrokenHeaderCorrectException' (60% confidence)
test/test_tarfile.py:1893: unused class 'CompressLevelRaises' (60% confidence)
test/test_tarfile.py:2168: unused variable 'test_create_taropen' (60% confidence)
test/test_tarfile.py:2169: unused variable 'test_create_existing_taropen' (60% confidence)
test/test_tarfile.py:2537: unused variable 'test_unicode_filename_error' (60% confidence)
test/test_tarfile.py:2573: unused variable 'test_append_compressed' (60% confidence)
test/test_tarfile.py:896: unused variable 'test_fail_comp' (60% confidence)
```

### edge-cases

Only vulture (1, first 10):
```
usage/all_and_strings.py:22: unused function 'g' (60% confidence)
```
Only condor (1, first 10):
```
decorators/decorators.py:6: unused class 'Widget' (60% confidence)
```

