use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct ServiceDependencies {
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub requires: Vec<String>,
    pub required_by: Vec<String>,
}

impl ServiceDependencies {
    pub fn new() -> Self {
        Self {
            before: Vec::new(),
            after: Vec::new(),
            requires: Vec::new(),
            required_by: Vec::new(),
        }
    }
}

pub struct DependencyResolver {
    services: HashMap<String, ServiceDependencies>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    pub fn add_service(&mut self, name: String, deps: ServiceDependencies) {
        self.services.insert(name, deps);
    }

    /// Compute the startup order respecting dependencies
    pub fn compute_startup_order(&self) -> Result<Vec<String>, String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize
        for service_name in self.services.keys() {
            in_degree.insert(service_name.clone(), 0);
            graph.insert(service_name.clone(), Vec::new());
        }

        // Build dependency graph
        for (service_name, deps) in &self.services {
            // Handle "After" - this service starts after others
            for after in &deps.after {
                if self.services.contains_key(after) {
                    graph.get_mut(after).unwrap().push(service_name.clone());
                    *in_degree.get_mut(service_name).unwrap() += 1;
                }
            }

            // Handle "Before" - others start after this service
            for before in &deps.before {
                if self.services.contains_key(before) {
                    graph.get_mut(service_name).unwrap().push(before.clone());
                    *in_degree.get_mut(before).unwrap() += 1;
                }
            }

            // Handle "Requires" - must start after required services
            for required in &deps.requires {
                if self.services.contains_key(required) {
                    graph.get_mut(required).unwrap().push(service_name.clone());
                    *in_degree.get_mut(service_name).unwrap() += 1;
                }
            }
        }

        // Topological sort using Kahn's algorithm
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut result: Vec<String> = Vec::new();

        // Find all nodes with no incoming edges
        for (service, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(service.clone());
            }
        }

        while let Some(service) = queue.pop_front() {
            result.push(service.clone());

            if let Some(neighbors) = graph.get(&service) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.services.len() {
            return Err("Circular dependency detected in service definitions".to_string());
        }

        Ok(result)
    }

    /// Check if all required dependencies exist
    pub fn validate_dependencies(&self) -> Result<(), String> {
        for (service_name, deps) in &self.services {
            for required in &deps.requires {
                if !self.services.contains_key(required) {
                    return Err(format!(
                        "Service '{}' requires '{}' which does not exist",
                        service_name, required
                    ));
                }
            }

            for after in &deps.after {
                if !self.services.contains_key(after) {
                    eprintln!(
                        "[WARN] Service '{}' has After='{}' which does not exist (non-fatal)",
                        service_name, after
                    );
                }
            }

            for before in &deps.before {
                if !self.services.contains_key(before) {
                    eprintln!(
                        "[WARN] Service '{}' has Before='{}' which does not exist (non-fatal)",
                        service_name, before
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_order() {
        let mut resolver = DependencyResolver::new();

        let mut deps_a = ServiceDependencies::new();
        deps_a.before.push("b".to_string());
        resolver.add_service("a".to_string(), deps_a);

        resolver.add_service("b".to_string(), ServiceDependencies::new());

        let order = resolver.compute_startup_order().unwrap();
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();

        assert!(pos_a < pos_b);
    }

    #[test]
    fn test_requires() {
        let mut resolver = DependencyResolver::new();

        resolver.add_service("db".to_string(), ServiceDependencies::new());

        let mut deps_app = ServiceDependencies::new();
        deps_app.requires.push("db".to_string());
        resolver.add_service("app".to_string(), deps_app);

        let order = resolver.compute_startup_order().unwrap();
        let pos_db = order.iter().position(|x| x == "db").unwrap();
        let pos_app = order.iter().position(|x| x == "app").unwrap();

        assert!(pos_db < pos_app);
    }

    #[test]
    fn test_circular_dependency() {
        let mut resolver = DependencyResolver::new();

        let mut deps_a = ServiceDependencies::new();
        deps_a.after.push("b".to_string());
        resolver.add_service("a".to_string(), deps_a);

        let mut deps_b = ServiceDependencies::new();
        deps_b.after.push("a".to_string());
        resolver.add_service("b".to_string(), deps_b);

        assert!(resolver.compute_startup_order().is_err());
    }
}
