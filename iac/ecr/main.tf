resource "aws_ecr_repository" "repository" {
  name                 = "fhir-example"

  image_tag_mutability = "IMMUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_ecr_lifecycle_policy" "retention" {
  repository = aws_ecr_repository.repository.name
  policy     = jsonencode({
    rules = [
      {
	rulePriority = 1
	description = "Keep last 2 images"
	selection = {
	  tagStatus = "tagged"
	  tagPatternList = [ "*" ]
	  countType = "imageCountMoreThan",
	  countNumber = 2
	}
	action = {
	  type = "expire"
	}
      }
    ]
  })
}

resource "aws_ecr_registry_scanning_configuration" "scan_configuration" {
  scan_type = "ENHANCED"

  rule {
    scan_frequency = "SCAN_ON_PUSH"
    repository_filter {
      filter      = "*"
      filter_type = "WILDCARD"
    }
  }
}
