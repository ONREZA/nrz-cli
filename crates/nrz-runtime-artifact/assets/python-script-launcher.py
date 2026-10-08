import os
import runpy
import sys

application_root, dependency_root, entry, *arguments = sys.argv[1:]
sys.argv = [entry, *arguments]
sys.path[0] = os.path.dirname(entry)
onreza_add_site_packages(dependency_root, (os.path.dirname(entry), application_root, os.path.join(application_root, "src")))
# Initialize wheel paths before standard sitecustomize imports them. Restore
# qualified prefix sites without processing the staged tree a second time.
site.main()
onreza_add_site_packages(dependency_root, (os.path.dirname(entry), application_root, os.path.join(application_root, "src")))
runpy.run_path(entry, run_name="__main__")
